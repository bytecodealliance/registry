//! A module for Warg registry API clients.

use anyhow::{anyhow, bail, Context, Result};
use bytes::Bytes;
use futures_util::{Stream, TryStreamExt};
use reqwest::{Body, IntoUrl, Response, StatusCode, Url};
use serde::de::DeserializeOwned;
use thiserror::Error;
use url::Host;
use warg_api::{
    content::{ContentError, ContentSource},
    fetch::{CheckpointResponse, FetchError, FetchRequest, FetchResponse},
    package::{PackageError, PendingRecordResponse, PublishRequest, RecordResponse},
    proof::{InclusionRequest, InclusionResponse, ProofError},
};
use warg_crypto::hash::{DynHash, Sha256};
use warg_protocol::{
    registry::{LogLeaf, MapCheckpoint, MapLeaf},
    ProtoEnvelopeBody,
};
use warg_transparency::{log::LogProofBundle, map::MapProofBundle};

/// Represents an error from the Warg API.
#[derive(Debug, Error)]
pub enum ApiError {
    /// An error from the content API.
    #[error(transparent)]
    Content(#[from] ContentError),
    /// An error from the fetch API.
    #[error(transparent)]
    Fetch(#[from] FetchError),
    /// An error from the package API.
    #[error(transparent)]
    Package(#[from] PackageError),
    /// An error from the proof API.
    #[error(transparent)]
    Proof(#[from] ProofError),
    /// Failed to send a request to the API.
    #[error("failed to send API request: {0}")]
    Request(#[from] reqwest::Error),
    /// The API returned JSON that could not be deserialized by the client.
    #[error("failed to deserialize JSON: {message} (status code: {status})")]
    DeserializationFailed {
        /// The status code of the response.
        status: StatusCode,
        /// The deserialization error message.
        message: String,
    },
    /// An error was encountered in the client.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    /// The API returned an unknown error.
    #[error("the server returned an unknown response (status code: {0})")]
    Unknown(StatusCode),
}

/// Represents an result from the Warg API client.
pub type ApiResult<T> = Result<T, ApiError>;

async fn deserialize<T: DeserializeOwned>(response: Response) -> ApiResult<T> {
    let status = response.status();
    match response.headers().get("content-type") {
        Some(content_type) if content_type == "application/json" => {
            match response.json::<T>().await {
                Ok(e) => Ok(e),
                Err(e) => Err(ApiError::DeserializationFailed {
                    status,
                    message: e.to_string(),
                }),
            }
        }
        _ => Err(ApiError::Unknown(status)),
    }
}

async fn into_result<T: DeserializeOwned, E: DeserializeOwned + Into<ApiError>>(
    response: Response,
) -> ApiResult<T> {
    if response.status().is_success() {
        return deserialize::<T>(response).await;
    }

    Err(deserialize::<E>(response).await?.into())
}

/// Represents a Warg API client for communicating with
/// a Warg registry server.
pub struct Client(Url);

impl Client {
    /// Creates a new API client with the given URL.
    pub fn new(url: impl IntoUrl) -> Result<Self> {
        let url = url.into_url()?;
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

        Ok(Self(url))
    }

    /// Gets the latest checkpoint from the registry.
    pub async fn latest_checkpoint(&self) -> ApiResult<CheckpointResponse> {
        let url = self.0.join("fetch/checkpoint").unwrap();
        tracing::debug!("getting latest checkpoint at `{url}`");
        into_result::<_, FetchError>(reqwest::get(url).await?).await
    }

    /// Fetches package log entries from the registry.
    pub async fn fetch_logs(&self, request: FetchRequest) -> ApiResult<FetchResponse> {
        let client = reqwest::Client::new();
        let response = client
            .post(self.0.join("fetch/logs").unwrap())
            .json(&request)
            .send()
            .await?;

        into_result::<_, FetchError>(response).await
    }

    /// Publishes a new package record to the registry.
    pub async fn publish(
        &self,
        package_name: &str,
        record: ProtoEnvelopeBody,
        content_sources: Vec<ContentSource>,
    ) -> ApiResult<PendingRecordResponse> {
        let client = reqwest::Client::new();
        let request = PublishRequest {
            name: package_name.to_string(),
            record,
            content_sources,
        };

        let url = self.0.join("package").unwrap();
        tracing::debug!("publishing package `{package_name}` to `{url}`");
        into_result::<_, PackageError>(client.post(url).json(&request).send().await?).await
    }

    /// Gets the pending package record from the registry.
    pub async fn get_pending_package_record(
        &self,
        route: &str,
    ) -> ApiResult<PendingRecordResponse> {
        let url = self.0.join(route).unwrap();
        tracing::debug!("getting pending package record from `{url}`");
        into_result::<_, PackageError>(reqwest::get(url).await?).await
    }

    /// Gets the package record from the registry.
    pub async fn get_package_record(&self, route: &str) -> ApiResult<RecordResponse> {
        let url = self.0.join(route).unwrap();
        tracing::debug!("getting package record from `{url}`");
        into_result::<_, PackageError>(reqwest::get(url).await?).await
    }

    /// Proves the inclusion of the given package log heads in the registry.
    pub async fn prove_inclusion(
        &self,
        checkpoint: &MapCheckpoint,
        heads: Vec<LogLeaf>,
    ) -> ApiResult<()> {
        let client = reqwest::Client::new();
        let request = InclusionRequest {
            checkpoint: checkpoint.clone(),
            heads: heads.clone(),
        };

        let url = self.0.join("proof/inclusion").unwrap();
        tracing::debug!("proving checkpoint inclusion from `{url}`");
        let response = into_result::<InclusionResponse, ProofError>(
            client.post(url).json(&request).send().await?,
        )
        .await?;

        let log_proof_bundle: LogProofBundle<Sha256, LogLeaf> =
            LogProofBundle::decode(response.log.as_slice())?;
        let (log_data, _, log_inclusions) = log_proof_bundle.unbundle();
        for (leaf, proof) in heads.iter().zip(log_inclusions.iter()) {
            let found = proof
                .evaluate_value(&log_data, leaf)
                .map_err(|e| ApiError::Other(anyhow!(e)))?;
            let root = checkpoint.log_root.clone().try_into()?;
            if found != root {
                return Err(ApiError::Proof(ProofError::IncorrectProof { root, found }));
            }
        }

        let map_proof_bundle: MapProofBundle<Sha256, MapLeaf> =
            MapProofBundle::decode(response.map.as_slice())?;
        let map_inclusions = map_proof_bundle.unbundle();
        for (leaf, proof) in heads.into_iter().zip(map_inclusions.iter()) {
            let found = proof.evaluate(
                &leaf.log_id,
                &MapLeaf {
                    record_id: leaf.record_id.clone(),
                },
            );
            let root = checkpoint.map_root.clone().try_into()?;
            if found != root {
                return Err(ApiError::Proof(ProofError::IncorrectProof { root, found }));
            }
        }

        Ok(())
    }

    /// Proves consistency of a new checkpoint with a previously known checkpoint.
    pub async fn prove_log_consistency(
        &self,
        old_root: DynHash,
        new_root: DynHash,
    ) -> ApiResult<()> {
        dbg!(old_root);
        dbg!(new_root);
        todo!()
    }

    /// Uploads package content to the registry.
    pub async fn upload_content(
        &self,
        digest: &DynHash,
        content: impl Into<Body>,
    ) -> ApiResult<String> {
        let client = reqwest::Client::new();

        let url = self.content_url(digest);
        tracing::debug!("checking if content exists at `{url}`");
        if client.head(&url).send().await?.status().is_success() {
            return Ok(url);
        }

        tracing::debug!("uploading content to `{url}`");

        let url = self.0.join("content").unwrap();
        let response = client.post(url).body(content).send().await?;
        if !response.status().is_success() {
            return Err(deserialize::<ContentError>(response).await?.into());
        }

        let location = response
            .headers()
            .get("location")
            .context("Uploaded URL not known")?
            .to_str()
            .map_err(|e| anyhow!(e))?;

        Ok(self.0.join(location).map_err(|e| anyhow!(e))?.to_string())
    }

    /// Downloads package content from the registry.
    pub async fn download_content(
        &self,
        digest: &DynHash,
    ) -> ApiResult<impl Stream<Item = Result<Bytes>>> {
        let url = self.content_url(digest);

        tracing::debug!("downloading content from `{url}`");

        let response = reqwest::get(url).await?;
        if !response.status().is_success() {
            return Err(deserialize::<ContentError>(response).await?.into());
        }

        Ok(response.bytes_stream().map_err(|e| anyhow!(e)))
    }

    fn content_url(&self, digest: &DynHash) -> String {
        format!(
            "{base}/{digest}",
            base = self.0.join("content").unwrap(),
            digest = digest.to_string().replace(':', "-")
        )
    }
}
