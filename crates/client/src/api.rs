//! A module for Warg registry API clients.

use anyhow::{anyhow, bail, Context, Result};
use bytes::Bytes;
use futures_util::{Stream, TryStreamExt};
use reqwest::{Body, IntoUrl, Response, StatusCode, Url};
use serde::de::DeserializeOwned;
use thiserror::Error;
use url::Host;
use warg_api::v1::{
    fetch::{FetchError, FetchLogsRequest, FetchLogsResponse},
    package::{
        ContentSource, PackageError, PackageRecord, PackageRecordState, PublishRecordRequest,
    },
    paths,
    proof::{
        ConsistencyRequest, ConsistencyResponse, InclusionRequest, InclusionResponse, ProofError,
    },
};
use warg_crypto::hash::{AnyHash, HashError, Sha256};
use warg_protocol::{
    registry::{LogId, LogLeaf, MapCheckpoint, MapLeaf, RecordId},
    SerdeEnvelope,
};
use warg_transparency::{
    log::{ConsistencyProofError, InclusionProofError, LogProofBundle, ProofBundle},
    map::MapProofBundle,
};

/// Represents an error that occurred while communicating with the registry.
#[derive(Debug, Error)]
pub enum ClientError {
    /// An error was returned from the fetch API.
    #[error(transparent)]
    Fetch(#[from] FetchError),
    /// An error was returned from the package API.
    #[error(transparent)]
    Package(#[from] PackageError),
    /// An error was returned from the proof API.
    #[error(transparent)]
    Proof(#[from] ProofError),
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
    /// An other error occurred during the requested operation.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

async fn deserialize<T: DeserializeOwned>(response: Response) -> Result<T, ClientError> {
    let status = response.status();
    match response.headers().get("content-type") {
        Some(content_type) if content_type == "application/json" => {
            match response.json::<T>().await {
                Ok(e) => Ok(e),
                Err(e) => Err(ClientError::UnexpectedResponse {
                    status,
                    message: format!("failed to deserialize JSON response: {e}"),
                }),
            }
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
    url: Url,
    client: reqwest::Client,
}

impl Client {
    /// Creates a new API client with the given URL.
    pub fn new(url: impl IntoUrl) -> Result<Self> {
        let url = Self::validate_url(url)?;
        Ok(Self {
            url,
            client: reqwest::Client::new(),
        })
    }

    /// Gets the URL of the API client.
    pub fn url(&self) -> &str {
        self.url.as_str()
    }

    /// Parses and validates the given URL.
    ///
    /// Returns the validated URL on success.
    pub fn validate_url(url: impl IntoUrl) -> Result<Url> {
        // Default to a HTTPS scheme if none is provided
        let url: Url = if !url.as_str().contains("://") {
            Url::parse(&format!("https://{url}", url = url.as_str()))
                .context("failed to parse registry server URL")?
        } else {
            url.into_url()
                .context("failed to parse registry server URL")?
        };

        match url.scheme() {
            "https" => {}
            "http" => {
                // Only allow HTTP connections to loopback
                match url
                    .host()
                    .ok_or_else(|| anyhow!("expected a host for URL `{url}`"))?
                {
                    Host::Domain(d) => {
                        if d != "localhost" {
                            bail!("an unsecured connection is not permitted to `{d}`");
                        }
                    }
                    Host::Ipv4(ip) => {
                        if !ip.is_loopback() {
                            bail!("an unsecured connection is not permitted to address `{ip}`");
                        }
                    }
                    Host::Ipv6(ip) => {
                        if !ip.is_loopback() {
                            bail!("an unsecured connection is not permitted to address `{ip}`");
                        }
                    }
                }
            }
            _ => bail!("expected a HTTPS scheme for URL `{url}`"),
        }
        Ok(url)
    }

    /// Gets the latest checkpoint from the registry.
    pub async fn latest_checkpoint(&self) -> Result<SerdeEnvelope<MapCheckpoint>, ClientError> {
        let url = self.url.join(paths::fetch_checkpoint()).unwrap();
        tracing::debug!("getting latest checkpoint at `{url}`");
        into_result::<_, FetchError>(reqwest::get(url).await?).await
    }

    /// Fetches package log entries from the registry.
    pub async fn fetch_logs(
        &self,
        request: FetchLogsRequest<'_>,
    ) -> Result<FetchLogsResponse, ClientError> {
        let url = self.url.join(paths::fetch_logs()).unwrap();
        tracing::debug!("fetching logs at `{url}`");

        let response = self.client.post(url).json(&request).send().await?;
        into_result::<_, FetchError>(response).await
    }

    /// Publish a new record to a package log.
    pub async fn publish_package_record(
        &self,
        log_id: &LogId,
        request: PublishRecordRequest<'_>,
    ) -> Result<PackageRecord, ClientError> {
        let url = self
            .url
            .join(&paths::publish_package_record(log_id))
            .unwrap();
        tracing::debug!(
            "appending record to package `{id}` at `{url}`",
            id = request.id
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
        let url = self
            .url
            .join(&paths::package_record(log_id, record_id))
            .unwrap();
        tracing::debug!("getting record `{record_id}` for package `{log_id}` at `{url}`");

        let response = reqwest::get(url).await?;
        into_result::<_, PackageError>(response).await
    }

    /// Downloads the content associated with a given record.
    pub async fn download_content(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        digest: &AnyHash,
    ) -> Result<impl Stream<Item = Result<Bytes>>, ClientError> {
        tracing::debug!("fetching record `{record_id}` for package `{log_id}`");

        let record = self.get_package_record(log_id, record_id).await?;
        let sources = match &record.state {
            PackageRecordState::Published {
                content_sources, ..
            } => content_sources
                .get(digest)
                .ok_or_else(|| ClientError::NoSourceForContent(digest.clone()))?,
            _ => {
                return Err(ClientError::RecordNotPublished(record_id.clone()));
            }
        };

        for source in sources {
            let url = match source {
                ContentSource::Http { url } => url,
            };

            tracing::debug!("downloading content `{digest}` from `{url}`");

            let response = reqwest::get(url).await?;
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
    pub async fn prove_inclusion(&self, request: InclusionRequest<'_>) -> Result<(), ClientError> {
        let url = self.url.join(paths::prove_inclusion()).unwrap();
        tracing::debug!("proving checkpoint inclusion at `{url}`");

        let response = into_result::<InclusionResponse, ProofError>(
            self.client.post(url).json(&request).send().await?,
        )
        .await?;

        Self::validate_inclusion_response(
            response,
            request.checkpoint.as_ref(),
            request.leafs.as_ref(),
        )
    }

    /// Proves consistency between two log roots.
    pub async fn prove_log_consistency(
        &self,
        request: ConsistencyRequest<'_>,
    ) -> Result<(), ClientError> {
        let url = self.url.join(paths::prove_consistency()).unwrap();
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

        if request.from.as_ref() != &from {
            return Err(ClientError::IncorrectConsistencyProof {
                root: request.from.into_owned(),
                found: from,
            });
        }

        if request.to.as_ref() != &to {
            return Err(ClientError::IncorrectConsistencyProof {
                root: request.to.into_owned(),
                found: to,
            });
        }

        Ok(())
    }

    /// Uploads package content to the registry.
    pub async fn upload_content(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        digest: &AnyHash,
        content: impl Into<Body>,
    ) -> Result<String, ClientError> {
        let url = self
            .url
            .join(&paths::package_record_content(log_id, record_id, digest))
            .unwrap();
        tracing::debug!("uploading content to `{url}`");

        let response = self.client.post(url).body(content).send().await?;
        if !response.status().is_success() {
            return Err(ClientError::Package(
                deserialize::<PackageError>(response).await?,
            ));
        }

        Ok(response
            .headers()
            .get("location")
            .ok_or_else(|| ClientError::UnexpectedResponse {
                status: response.status(),
                message: "location header missing from response".into(),
            })?
            .to_str()
            .map_err(|_| ClientError::UnexpectedResponse {
                status: response.status(),
                message: "returned location header was not UTF-8".into(),
            })?
            .to_string())
    }

    fn validate_inclusion_response(
        response: InclusionResponse,
        checkpoint: &MapCheckpoint,
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
