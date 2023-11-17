//! A module for Warg registry API clients.

use anyhow::{anyhow, Result};
use bytes::Bytes;
use futures_util::{Stream, TryStreamExt};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Body, IntoUrl, Method, Response, StatusCode,
};
use serde::de::DeserializeOwned;
use std::{borrow::Cow, collections::HashMap};
use thiserror::Error;
use warg_api::v1::{
    content::{ContentError, ContentSourcesResponse},
    fetch::{
        FetchError, FetchLogsRequest, FetchLogsResponse, FetchPackageNamesRequest,
        FetchPackageNamesResponse,
    },
    ledger::{LedgerError, LedgerSourcesResponse},
    monitor::{CheckpointVerificationResponse, MonitorError},
    package::{ContentSource, PackageError, PackageRecord, PublishRecordRequest},
    paths,
    proof::{
        ConsistencyRequest, ConsistencyResponse, InclusionRequest, InclusionResponse, ProofError,
    },
};
use warg_crypto::hash::{AnyHash, HashError, Sha256};
use warg_protocol::{
    registry::{Checkpoint, LogId, LogLeaf, MapLeaf, RecordId, TimestampedCheckpoint},
    SerdeEnvelope,
};
use warg_transparency::{
    log::{ConsistencyProofError, InclusionProofError, LogProofBundle, ProofBundle},
    map::MapProofBundle,
};

use crate::registry_url::RegistryUrl;

/// Represents an error that occurred while communicating with the registry.
#[derive(Debug, Error)]
pub enum ClientError {
    /// An error was returned from the fetch API.
    #[error(transparent)]
    Fetch(#[from] FetchError),
    /// An error was returned from the package API.
    #[error(transparent)]
    Package(#[from] PackageError),
    /// An error was returned from the content API.
    #[error(transparent)]
    Content(#[from] ContentError),
    /// An error was returned from the proof API.
    #[error(transparent)]
    Proof(#[from] ProofError),
    /// An error was returned from the monitor API.
    #[error(transparent)]
    Monitor(#[from] MonitorError),
    /// An error was returned from the ledger API.
    #[error(transparent)]
    Ledger(#[from] LedgerError),
    /// An error occurred while communicating with the registry.
    #[error("failed to send request to registry server: {0}")]
    Communication(#[from] reqwest::Error),
    /// An unexpected response was received from the server.
    #[error("{message} (status code: {status})")]
    UnexpectedResponse {
        /// The response from the server.
        status: StatusCode,
        /// The error message.
        message: String,
    },
    /// The provided root for a consistency proof was incorrect.
    #[error(
        "the client failed to prove consistency: found root `{found}` but was given root `{root}`"
    )]
    IncorrectConsistencyProof {
        /// The provided root.
        root: AnyHash,
        /// The found root.
        found: AnyHash,
    },
    /// A hash returned from the server was incorrect.
    #[error("the server returned an invalid hash: {0}")]
    Hash(#[from] HashError),
    /// The client failed a consistency proof.
    #[error("the client failed a consistency proof: {0}")]
    ConsistencyProof(#[from] ConsistencyProofError),
    /// The client failed an inclusion proof.
    #[error("the client failed an inclusion proof: {0}")]
    InclusionProof(#[from] InclusionProofError),
    /// The record was not published.
    #[error("record `{0}` has not been published")]
    RecordNotPublished(RecordId),
    /// Could not find a source for the given content digest.
    #[error("no download location could be found for content digest `{0}`")]
    NoSourceForContent(AnyHash),
    /// All sources for the given content digest returned an error response.
    #[error("all sources for content digest `{0}` returned an error response")]
    AllSourcesFailed(AnyHash),
    /// Invalid upload HTTP method.
    #[error("server returned an invalid HTTP method `{0}`")]
    InvalidHttpMethod(String),
    /// Invalid upload HTTP method.
    #[error("server returned an invalid HTTP header `{0}: {1}`")]
    InvalidHttpHeader(String, String),
    /// An other error occurred during the requested operation.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

async fn deserialize<T: DeserializeOwned>(response: Response) -> Result<T, ClientError> {
    let status = response.status();
    match response.headers().get("content-type") {
        Some(content_type) if content_type == "application/json" => {
            let bytes = response
                .bytes()
                .await
                .map_err(|e| ClientError::UnexpectedResponse {
                    status,
                    message: format!("failed to read response: {e}"),
                })?;
            serde_json::from_slice(&bytes).map_err(|e| {
                tracing::debug!(
                    "Unexpected response body: {}",
                    String::from_utf8_lossy(&bytes)
                );
                ClientError::UnexpectedResponse {
                    status,
                    message: format!("failed to deserialize JSON response: {e}"),
                }
            })
        }
        Some(ty) => Err(ClientError::UnexpectedResponse {
            status,
            message: format!(
                "the server returned an unsupported content type of `{ty}`",
                ty = ty.to_str().unwrap_or("")
            ),
        }),
        None => Err(ClientError::UnexpectedResponse {
            status,
            message: "the server response did not include a content type header".into(),
        }),
    }
}

async fn into_result<T: DeserializeOwned, E: DeserializeOwned + Into<ClientError>>(
    response: Response,
) -> Result<T, ClientError> {
    if response.status().is_success() {
        deserialize::<T>(response).await
    } else {
        Err(deserialize::<E>(response).await?.into())
    }
}

/// Represents a Warg API client for communicating with
/// a Warg registry server.
pub struct Client {
    url: RegistryUrl,
    client: reqwest::Client,
}

impl Client {
    /// Creates a new API client with the given URL.
    pub fn new(url: impl IntoUrl) -> Result<Self> {
        let url = RegistryUrl::new(url)?;
        Ok(Self {
            url,
            client: reqwest::Client::new(),
        })
    }

    /// Gets the URL of the API client.
    pub fn url(&self) -> &RegistryUrl {
        &self.url
    }

    /// Gets the latest checkpoint from the registry.
    pub async fn latest_checkpoint(
        &self,
    ) -> Result<SerdeEnvelope<TimestampedCheckpoint>, ClientError> {
        let url = self.url.join(paths::fetch_checkpoint());
        tracing::debug!("getting latest checkpoint at `{url}`");
        into_result::<_, FetchError>(reqwest::get(url).await?).await
    }

    /// Verify checkpoint of the registry.
    pub async fn verify_checkpoint(
        &self,
        request: SerdeEnvelope<TimestampedCheckpoint>,
    ) -> Result<CheckpointVerificationResponse, ClientError> {
        let url = self.url.join(paths::verify_checkpoint());
        tracing::debug!("verifying checkpoint at `{url}`");

        let response = self.client.post(url).json(&request).send().await?;
        into_result::<_, MonitorError>(response).await
    }

    /// Fetches package log entries from the registry.
    pub async fn fetch_logs(
        &self,
        request: FetchLogsRequest<'_>,
    ) -> Result<FetchLogsResponse, ClientError> {
        let url = self.url.join(paths::fetch_logs());
        tracing::debug!("fetching logs at `{url}`");

        let response = self.client.post(url).json(&request).send().await?;
        into_result::<_, FetchError>(response).await
    }

    /// Fetches package names from the registry.
    pub async fn fetch_package_names(
        &self,
        request: FetchPackageNamesRequest<'_>,
    ) -> Result<FetchPackageNamesResponse, ClientError> {
        let url = self.url.join(paths::fetch_package_names());
        tracing::debug!("fetching package names at `{url}`");

        let response = self.client.post(url).json(&request).send().await?;
        into_result::<_, FetchError>(response).await
    }

    /// Gets ledger sources from the registry.
    pub async fn ledger_sources(&self) -> Result<LedgerSourcesResponse, ClientError> {
        let url = self.url.join(paths::ledger_sources());
        tracing::debug!("getting ledger sources at `{url}`");

        let response = reqwest::get(url).await?;
        into_result::<_, LedgerError>(response).await
    }

    /// Publish a new record to a package log.
    pub async fn publish_package_record(
        &self,
        log_id: &LogId,
        request: PublishRecordRequest<'_>,
    ) -> Result<PackageRecord, ClientError> {
        let url = self.url.join(&paths::publish_package_record(log_id));
        tracing::debug!(
            "appending record to package `{name}` at `{url}`",
            name = request.package_name
        );

        let response = self.client.post(url).json(&request).send().await?;
        into_result::<_, PackageError>(response).await
    }

    /// Gets a package record from the registry.
    pub async fn get_package_record(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> Result<PackageRecord, ClientError> {
        let url = self.url.join(&paths::package_record(log_id, record_id));
        tracing::debug!("getting record `{record_id}` for package `{log_id}` at `{url}`");

        let response = reqwest::get(url).await?;
        into_result::<_, PackageError>(response).await
    }

    /// Gets a content sources from the registry.
    pub async fn content_sources(
        &self,
        digest: &AnyHash,
    ) -> Result<ContentSourcesResponse, ClientError> {
        let url = self.url.join(&paths::content_sources(digest));
        tracing::debug!("getting content sources for digest `{digest}` at `{url}`");

        let response = reqwest::get(url).await?;
        into_result::<_, ContentError>(response).await
    }

    /// Downloads the content associated with a given record.
    pub async fn download_content(
        &self,
        digest: &AnyHash,
    ) -> Result<impl Stream<Item = Result<Bytes>>, ClientError> {
        tracing::debug!("requesting content download for digest `{digest}`");

        let ContentSourcesResponse { content_sources } = self.content_sources(digest).await?;

        let sources = content_sources
            .get(digest)
            .ok_or(ClientError::AllSourcesFailed(digest.clone()))?;

        for source in sources {
            let ContentSource::HttpGet { url, .. } = source;

            tracing::debug!("downloading content `{digest}` from `{url}`");

            let response = self.client.get(url).send().await?;
            if !response.status().is_success() {
                tracing::debug!(
                    "failed to download content `{digest}` from `{url}`: {status}",
                    status = response.status()
                );
                continue;
            }

            return Ok(response.bytes_stream().map_err(|e| anyhow!(e)));
        }

        Err(ClientError::AllSourcesFailed(digest.clone()))
    }

    /// Proves the inclusion of the given package log heads in the registry.
    pub async fn prove_inclusion(
        &self,
        request: InclusionRequest,
        checkpoint: &Checkpoint,
        leafs: &[LogLeaf],
    ) -> Result<(), ClientError> {
        let url = self.url.join(paths::prove_inclusion());
        tracing::debug!("proving checkpoint inclusion at `{url}`");

        let response = into_result::<InclusionResponse, ProofError>(
            self.client.post(url).json(&request).send().await?,
        )
        .await?;

        Self::validate_inclusion_response(response, checkpoint, leafs)
    }

    /// Proves consistency between two log roots.
    pub async fn prove_log_consistency(
        &self,
        request: ConsistencyRequest,
        from_log_root: Cow<'_, AnyHash>,
        to_log_root: Cow<'_, AnyHash>,
    ) -> Result<(), ClientError> {
        let url = self.url.join(paths::prove_consistency());
        let response = into_result::<ConsistencyResponse, ProofError>(
            self.client.post(url).json(&request).send().await?,
        )
        .await?;

        let proof = ProofBundle::<Sha256, LogLeaf>::decode(&response.proof).unwrap();
        let (log_data, consistencies, inclusions) = proof.unbundle();
        if !inclusions.is_empty() {
            return Err(ClientError::Proof(ProofError::BundleFailure(
                "expected no inclusion proofs".into(),
            )));
        }

        if consistencies.len() != 1 {
            return Err(ClientError::Proof(ProofError::BundleFailure(
                "expected exactly one consistency proof".into(),
            )));
        }

        let (from, to) = consistencies
            .first()
            .unwrap()
            .evaluate(&log_data)
            .map(|(from, to)| (AnyHash::from(from), AnyHash::from(to)))?;

        if from_log_root.as_ref() != &from {
            return Err(ClientError::IncorrectConsistencyProof {
                root: from_log_root.into_owned(),
                found: from,
            });
        }

        if to_log_root.as_ref() != &to {
            return Err(ClientError::IncorrectConsistencyProof {
                root: to_log_root.into_owned(),
                found: to,
            });
        }

        Ok(())
    }

    /// Uploads package content to the registry.
    pub async fn upload_content(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        content: impl Into<Body>,
    ) -> Result<(), ClientError> {
        // Upload URLs may be relative to the registry URL.
        let url = self.url.join(url);

        let method = match method {
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            method => return Err(ClientError::InvalidHttpMethod(method.to_string())),
        };

        let headers = headers
            .iter()
            .map(|(k, v)| {
                let name = match k.as_str() {
                    "authorization" => reqwest::header::AUTHORIZATION,
                    "content-type" => reqwest::header::CONTENT_TYPE,
                    _ => return Err(ClientError::InvalidHttpHeader(k.to_string(), v.to_string())),
                };
                let value = HeaderValue::try_from(k)
                    .map_err(|_| ClientError::InvalidHttpHeader(k.to_string(), v.to_string()))?;
                Ok((name, value))
            })
            .collect::<Result<HeaderMap, ClientError>>()?;

        tracing::debug!("uploading content to `{url}`");

        let response = self
            .client
            .request(method, url)
            .headers(headers)
            .body(content)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(ClientError::Package(
                deserialize::<PackageError>(response).await?,
            ));
        }

        Ok(())
    }

    fn validate_inclusion_response(
        response: InclusionResponse,
        checkpoint: &Checkpoint,
        leafs: &[LogLeaf],
    ) -> Result<(), ClientError> {
        let log_proof_bundle: LogProofBundle<Sha256, LogLeaf> =
            LogProofBundle::decode(response.log.as_slice())?;
        let (log_data, _, log_inclusions) = log_proof_bundle.unbundle();
        for (leaf, proof) in leafs.iter().zip(log_inclusions.iter()) {
            let found = proof.evaluate_value(&log_data, leaf)?;
            let root = checkpoint.log_root.clone().try_into()?;
            if found != root {
                return Err(ClientError::Proof(ProofError::IncorrectProof {
                    root: checkpoint.log_root.clone(),
                    found: found.into(),
                }));
            }
        }

        let map_proof_bundle: MapProofBundle<Sha256, LogId, MapLeaf> =
            MapProofBundle::decode(response.map.as_slice())?;
        let map_inclusions = map_proof_bundle.unbundle();
        for (leaf, proof) in leafs.iter().zip(map_inclusions.iter()) {
            let found = proof.evaluate(
                &leaf.log_id,
                &MapLeaf {
                    record_id: leaf.record_id.clone(),
                },
            );
            let root = checkpoint.map_root.clone().try_into()?;
            if found != root {
                return Err(ClientError::Proof(ProofError::IncorrectProof {
                    root: checkpoint.map_root.clone(),
                    found: found.into(),
                }));
            }
        }

        Ok(())
    }
}
