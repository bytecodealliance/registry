//! A module for Warg registry API clients.

use anyhow::{anyhow, bail, Result};
use bytes::Bytes;
use futures_util::{Stream, TryStreamExt};
use reqwest::{Body, IntoUrl, Response, Url};
use serde::de::DeserializeOwned;
use url::Host;
use warg_api::{
    content::{ContentError, ContentResult, ContentSource},
    fetch::{CheckpointResponse, FetchError, FetchRequest, FetchResponse, FetchResult},
    package::{PackageError, PackageResult, PendingRecordResponse, PublishRequest, RecordResponse},
    proof::{InclusionRequest, InclusionResponse, ProofError, ProofResult},
    FromError,
};
use warg_crypto::hash::{DynHash, Sha256};
use warg_protocol::{
    registry::{LogLeaf, MapCheckpoint, MapLeaf},
    ProtoEnvelopeBody,
};
use warg_transparency::{log::LogProofBundle, map::MapProofBundle};

async fn deserialize<T: DeserializeOwned>(response: Response) -> Result<T, String> {
    let status = response.status();
    match response.headers().get("content-type") {
        Some(content_type) if content_type == "application/json" => {
            match response.json::<T>().await {
                Ok(e) => Ok(e),
                Err(e) => Err(format!(
                    "failed to deserialize JSON response: {e} (status code: {status})"
                )),
            }
        }
        Some(ty) => Err(format!(
            "the server returned an unsupported content type of `{ty}` (status code: {status})",
            ty = ty.to_str().unwrap_or("")
        )),
        None => Err(format!(
            "the server did not return a content type (status code: {status})"
        )),
    }
}

async fn into_result<T: DeserializeOwned, E: DeserializeOwned + From<String>>(
    response: Response,
) -> Result<T, E> {
    if response.status().is_success() {
        Ok(deserialize::<T>(response).await.map_err(E::from)?)
    } else {
        Err(deserialize::<E>(response).await.map_err(E::from)?)
    }
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
    pub async fn latest_checkpoint(&self) -> FetchResult<CheckpointResponse> {
        let url = self.0.join("fetch/checkpoint").unwrap();
        tracing::debug!("getting latest checkpoint at `{url}`");
        into_result(reqwest::get(url).await.map_err(FetchError::from_error)?).await
    }

    /// Fetches package log entries from the registry.
    pub async fn fetch_logs(&self, request: FetchRequest) -> FetchResult<FetchResponse> {
        let client = reqwest::Client::new();
        let response = client
            .post(self.0.join("fetch/logs").unwrap())
            .json(&request)
            .send()
            .await
            .map_err(FetchError::from_error)?;

        into_result(response).await
    }

    /// Publishes a new package record to the registry.
    pub async fn publish(
        &self,
        package_name: &str,
        record: ProtoEnvelopeBody,
        content_sources: Vec<ContentSource>,
    ) -> PackageResult<PendingRecordResponse> {
        let client = reqwest::Client::new();
        let request = PublishRequest {
            name: package_name.to_string(),
            record,
            content_sources,
        };

        let url = self.0.join("package").unwrap();
        tracing::debug!("publishing package `{package_name}` to `{url}`");
        into_result(
            client
                .post(url)
                .json(&request)
                .send()
                .await
                .map_err(PackageError::from_error)?,
        )
        .await
    }

    /// Gets the pending package record from the registry.
    pub async fn get_pending_package_record(
        &self,
        route: &str,
    ) -> PackageResult<PendingRecordResponse> {
        let url = self.0.join(route).unwrap();
        tracing::debug!("getting pending package record from `{url}`");
        into_result::<_, PackageError>(reqwest::get(url).await.map_err(PackageError::from_error)?)
            .await
    }

    /// Gets the package record from the registry.
    pub async fn get_package_record(&self, route: &str) -> PackageResult<RecordResponse> {
        let url = self.0.join(route).unwrap();
        tracing::debug!("getting package record from `{url}`");
        into_result::<_, PackageError>(reqwest::get(url).await.map_err(PackageError::from_error)?)
            .await
    }

    /// Proves the inclusion of the given package log heads in the registry.
    pub async fn prove_inclusion(
        &self,
        checkpoint: &MapCheckpoint,
        heads: Vec<LogLeaf>,
    ) -> ProofResult<()> {
        let client = reqwest::Client::new();
        let request = InclusionRequest {
            checkpoint: checkpoint.clone(),
            heads: heads.clone(),
        };

        let url = self.0.join("proof/inclusion").unwrap();
        tracing::debug!("proving checkpoint inclusion from `{url}`");
        let response = into_result::<InclusionResponse, ProofError>(
            client
                .post(url)
                .json(&request)
                .send()
                .await
                .map_err(ProofError::from_error)?,
        )
        .await?;

        match Self::validate_inclusion_response(response, checkpoint, &heads) {
            Ok(()) => Ok(()),
            Err(e) => match e.downcast::<ProofError>() {
                Ok(e) => Err(e),
                Err(e) => Err(ProofError::from(e.to_string())),
            },
        }
    }

    /// Proves consistency of a new checkpoint with a previously known checkpoint.
    pub async fn prove_log_consistency(
        &self,
        old_root: DynHash,
        new_root: DynHash,
    ) -> ProofResult<()> {
        dbg!(old_root);
        dbg!(new_root);
        todo!()
    }

    /// Uploads package content to the registry.
    pub async fn upload_content(
        &self,
        digest: &DynHash,
        content: impl Into<Body>,
    ) -> ContentResult<String> {
        let client = reqwest::Client::new();

        let url = self.content_url(digest);
        tracing::debug!("checking if content exists at `{url}`");
        if client
            .head(&url)
            .send()
            .await
            .map_err(ContentError::from_error)?
            .status()
            .is_success()
        {
            return Ok(url);
        }

        tracing::debug!("uploading content to `{url}`");

        let url = self.0.join("content").unwrap();
        let response = client
            .post(url)
            .body(content)
            .send()
            .await
            .map_err(ContentError::from_error)?;
        if !response.status().is_success() {
            return Err(deserialize::<ContentError>(response).await?);
        }

        let location = response
            .headers()
            .get("location")
            .ok_or_else(|| {
                ContentError::from("server did not return a location header".to_string())
            })?
            .to_str()
            .map_err(|_| {
                ContentError::from("returned location header was not UTF-8".to_string())
            })?;

        Ok(self
            .0
            .join(location)
            .map_err(|_| {
                ContentError::from("returned location header was not relative".to_string())
            })?
            .to_string())
    }

    /// Downloads package content from the registry.
    pub async fn download_content(
        &self,
        digest: &DynHash,
    ) -> ContentResult<impl Stream<Item = Result<Bytes>>> {
        let url = self.content_url(digest);

        tracing::debug!("downloading content from `{url}`");

        let response = reqwest::get(url).await.map_err(ContentError::from_error)?;
        if !response.status().is_success() {
            return Err(deserialize::<ContentError>(response).await?);
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

    fn validate_inclusion_response(
        response: InclusionResponse,
        checkpoint: &MapCheckpoint,
        heads: &[LogLeaf],
    ) -> Result<()> {
        let log_proof_bundle: LogProofBundle<Sha256, LogLeaf> =
            LogProofBundle::decode(response.log.as_slice())?;
        let (log_data, _, log_inclusions) = log_proof_bundle.unbundle();
        for (leaf, proof) in heads.iter().zip(log_inclusions.iter()) {
            let found = proof.evaluate_value(&log_data, leaf)?;
            let root = checkpoint.log_root.clone().try_into()?;
            if found != root {
                return Err(anyhow!(ProofError::IncorrectProof { root, found }));
            }
        }

        let map_proof_bundle: MapProofBundle<Sha256, MapLeaf> =
            MapProofBundle::decode(response.map.as_slice())?;
        let map_inclusions = map_proof_bundle.unbundle();
        for (leaf, proof) in heads.iter().zip(map_inclusions.iter()) {
            let found = proof.evaluate(
                &leaf.log_id,
                &MapLeaf {
                    record_id: leaf.record_id.clone(),
                },
            );
            let root = checkpoint.map_root.clone().try_into()?;
            if found != root {
                return Err(anyhow!(ProofError::IncorrectProof { root, found }));
            }
        }

        Ok(())
    }
}
