//! A module for Warg registry API clients.

use crate::ClientError;
use anyhow::{anyhow, bail, Context, Result};
use bytes::Bytes;
use futures_util::{Stream, TryStreamExt};
use reqwest::{Body, IntoUrl, Url};
use std::time::Duration;
use url::Host;
use warg_api::{
    content::ContentSource,
    fetch::{CheckpointResponse, FetchRequest, FetchResponse},
    package::{PendingRecordResponse, PublishRequest, RecordResponse},
    proof::{InclusionRequest, InclusionResponse},
};
use warg_crypto::hash::{DynHash, Sha256};
use warg_protocol::{
    registry::{LogId, LogLeaf, MapCheckpoint, MapLeaf},
    ProtoEnvelopeBody, SerdeEnvelope,
};
use warg_transparency::{log::LogProofBundle, map::MapProofBundle};

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
    pub async fn latest_checkpoint(&self) -> Result<SerdeEnvelope<MapCheckpoint>, ClientError> {
        let url = self.0.join("fetch/checkpoint").unwrap();
        tracing::debug!("getting latest checkpoint at `{url}`");

        let response = reqwest::get(url).await?;
        if !response.status().is_success() {
            return Err(ClientError::ApiError {
                registry: self.0.host_str().unwrap_or("").to_string(),
                status: response.status().as_u16(),
                body: response.text().await?,
            });
        }

        let response = response.json::<CheckpointResponse>().await?;
        Ok(response.checkpoint)
    }

    /// Fetches package log entries from the registry.
    pub async fn fetch_logs(&self, request: FetchRequest) -> Result<FetchResponse, ClientError> {
        let client = reqwest::Client::new();
        let response = client
            .post(self.0.join("fetch/logs").unwrap())
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ClientError::ApiError {
                registry: self.0.host_str().unwrap_or("").to_string(),
                status: response.status().as_u16(),
                body: response.text().await?,
            });
        }

        Ok(response.json::<FetchResponse>().await?)
    }

    /// Publishes a new package record to the registry.
    pub async fn publish(
        &self,
        package_name: &str,
        record: ProtoEnvelopeBody,
        content_sources: Vec<ContentSource>,
    ) -> Result<RecordResponse, ClientError> {
        let client = reqwest::Client::new();
        let request = PublishRequest {
            name: package_name.to_string(),
            record,
            content_sources,
        };

        let url = self.package_url(package_name);

        tracing::debug!("publishing package `{package_name}` to `{url}`");
        let response = client.post(url).json(&request).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::ApiError {
                registry: self.0.host_str().unwrap_or("").to_string(),
                status: response.status().as_u16(),
                body: response.text().await?,
            });
        }

        let mut response = response.json::<PendingRecordResponse>().await?;

        loop {
            match response {
                PendingRecordResponse::Published { record_url } => {
                    return self.get_package_record(&record_url).await
                }
                PendingRecordResponse::Rejected { reason } => {
                    return Err(ClientError::PublishRejected {
                        package: package_name.to_string(),
                        reason,
                    });
                }
                PendingRecordResponse::Processing { status_url } => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    response = self.get_pending_package_record(&status_url).await?;
                }
            }
        }
    }

    /// Gets the pending package record from the registry.
    pub async fn get_pending_package_record(
        &self,
        route: &str,
    ) -> Result<PendingRecordResponse, ClientError> {
        let url = self.0.join(route).unwrap();
        tracing::debug!("getting pending package record from `{url}`");
        let response = reqwest::get(url).await?;
        if !response.status().is_success() {
            return Err(ClientError::ApiError {
                registry: self.0.host_str().unwrap_or("").to_string(),
                status: response.status().as_u16(),
                body: response.text().await?,
            });
        }

        Ok(response.json::<PendingRecordResponse>().await?)
    }

    /// Gets the package record from the registry.
    pub async fn get_package_record(&self, route: &str) -> Result<RecordResponse, ClientError> {
        let url = self.0.join(route).unwrap();
        tracing::debug!("getting package record from `{url}`");
        let response = reqwest::get(url).await?;
        if !response.status().is_success() {
            return Err(ClientError::ApiError {
                registry: self.0.host_str().unwrap_or("").to_string(),
                status: response.status().as_u16(),
                body: response.text().await?,
            });
        }

        Ok(response.json::<RecordResponse>().await?)
    }

    /// Proves the inclusion of the given package log heads in the registry.
    pub async fn prove_inclusion(
        &self,
        checkpoint: &MapCheckpoint,
        heads: Vec<LogLeaf>,
    ) -> Result<(), ClientError> {
        let client = reqwest::Client::new();
        let request = InclusionRequest {
            checkpoint: checkpoint.clone(),
            heads: heads.clone(),
        };

        let url = self.0.join("proof/inclusion").unwrap();
        tracing::debug!("proving checkpoint inclusion from `{url}`");
        let response = client.post(url).json(&request).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::ApiError {
                registry: self.0.host_str().unwrap_or("").to_string(),
                status: response.status().as_u16(),
                body: response.text().await?,
            });
        }

        let response = response.json::<InclusionResponse>().await?;

        let log_proof_bundle: LogProofBundle<Sha256, LogLeaf> =
            LogProofBundle::decode(response.log.as_slice())?;
        let (log_data, _, log_inclusions) = log_proof_bundle.unbundle();
        for (leaf, proof) in heads.iter().zip(log_inclusions.iter()) {
            let root = proof.evaluate_value(&log_data, leaf)?;
            if checkpoint.log_root != root.clone().into() {
                return Err(ClientError::Other(anyhow!(
                    "verification proof failed: expected log root `{expected}` but found `{root}`",
                    expected = checkpoint.log_root,
                    root = root
                )));
            }
        }

        let map_proof_bundle: MapProofBundle<Sha256, MapLeaf> =
            MapProofBundle::decode(response.map.as_slice())?;
        let map_inclusions = map_proof_bundle.unbundle();
        for (leaf, proof) in heads.iter().zip(map_inclusions.iter()) {
            let root = proof.evaluate(
                &leaf.log_id,
                &MapLeaf {
                    record_id: leaf.record_id.clone(),
                },
            );
            if checkpoint.map_root != root.clone().into() {
                return Err(ClientError::Other(anyhow!(
                    "verification proof failed: expected map root `{expected}` but found `{root}`",
                    expected = checkpoint.map_root,
                    root = root
                )));
            }
        }

        Ok(())
    }

    /// Proves consistency of a new checkpoint with a previously known checkpoint.
    pub async fn prove_log_consistency(&self, old_root: DynHash, new_root: DynHash) -> Result<()> {
        dbg!(old_root);
        dbg!(new_root);
        todo!()
    }

    /// Uploads package content to the registry.
    pub async fn upload_content(
        &self,
        digest: &DynHash,
        content: impl Into<Body>,
    ) -> Result<String, ClientError> {
        let client = reqwest::Client::new();

        let url = self.content_url(digest);
        tracing::debug!("checking if content exists at `{url}`");
        if client.head(&url).send().await?.status().is_success() {
            return Ok(url);
        }

        let url = self.0.join("content").unwrap();
        tracing::debug!("uploading content to `{url}`");
        let response = client.post(url).body(content).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::ApiError {
                registry: self.0.host_str().unwrap_or("").to_string(),
                status: response.status().as_u16(),
                body: response.text().await?,
            });
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
    ) -> Result<impl Stream<Item = Result<Bytes>>, ClientError> {
        let url = self.content_url(digest);

        tracing::debug!("downloading content from `{url}`");

        let response = reqwest::get(url).await?;

        if !response.status().is_success() {
            return Err(ClientError::ApiError {
                registry: self.0.host_str().unwrap_or("").to_string(),
                status: response.status().as_u16(),
                body: response.text().await?,
            });
        }

        Ok(response.bytes_stream().map_err(|e| anyhow!(e)))
    }

    fn package_url(&self, package_name: &str) -> String {
        format!(
            "{base}/{id}",
            base = self.0.join("package").unwrap(),
            id = LogId::package_log::<Sha256>(package_name)
        )
    }

    fn content_url(&self, digest: &DynHash) -> String {
        format!("{base}/{digest}", base = self.0.join("content").unwrap())
    }
}
