use std::{path::Path, sync::Arc, time::Duration};

use anyhow::{Error, Result};
use futures_util::stream::StreamExt;
use forrest::{log::LogProofBundle, map::MapProofBundle};
use tempfile::{NamedTempFile, TempPath};
use tokio::io::AsyncWriteExt;
use warg_crypto::hash::{DynHash, Sha256, Hash, Digest};
use warg_protocol::{
    package,
    registry::{LogLeaf, MapCheckpoint, MapLeaf},
    ProtoEnvelope, SerdeEnvelope,
};
use warg_server::api::{
    fetch::CheckpointResponse,
    package::{PendingRecordResponse, PublishRequest, RecordResponse},
    proof::{InclusionRequest, InclusionResponse},
};

pub use warg_server::api::fetch::{FetchRequest, FetchResponse};

pub use warg_server::services::core::{ContentSource, ContentSourceKind};

pub struct Client {
    server_url: String,
}

impl Client {
    pub fn new(server_url: String) -> Self {
        Self { server_url }
    }

    fn endpoint(&self, route: &str) -> String {
        format!("{}{}", self.server_url, route)
    }

    pub async fn latest_checkpoint(&self) -> Result<SerdeEnvelope<MapCheckpoint>> {
        let response = reqwest::get(self.endpoint("/fetch/checkpoint")).await?;
        let response = response.json::<CheckpointResponse>().await?;
        Ok(response.checkpoint)
    }

    pub async fn fetch_logs(&self, request: FetchRequest) -> Result<FetchResponse> {
        let client = reqwest::Client::new();
        let response = client
            .post(self.endpoint("/fetch/logs"))
            .json(&request)
            .send()
            .await?;
        let response = response.json::<FetchResponse>().await?;
        Ok(response)
    }

    pub async fn publish(
        &self,
        package_name: &str,
        record: Arc<ProtoEnvelope<package::PackageRecord>>,
        content_sources: Vec<ContentSource>,
    ) -> Result<RecordResponse> {
        println!("Publishing {}", package_name);
        let client = reqwest::Client::new();
        let request = PublishRequest {
            record: record.as_ref().clone().into(),
            content_sources,
        };
        let url = format!("{}/package/{}", self.server_url, package_name);
        let response = client.post(url).json(&request).send().await?;
        let response = response.json::<PendingRecordResponse>().await?;

        let record_url = match response {
            PendingRecordResponse::Published { record_url } => record_url,
            PendingRecordResponse::Rejected { reason } => return Err(Error::msg(format!("Record rejected for {}", reason))),
            PendingRecordResponse::Processing { status_url } => {
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let response = self.get_pending_package_record(&status_url).await?;
                    match response {
                        PendingRecordResponse::Published { record_url } => break record_url,
                        PendingRecordResponse::Rejected { reason } => return Err(Error::msg(format!("Record rejected for {}", reason))),
                        PendingRecordResponse::Processing { .. } => {},
                    }
                }
            },
        };

        let record_info = self.get_package_record(&record_url).await?;

        Ok(record_info)
    }

    pub async fn get_pending_package_record(&self, route: &str) -> Result<PendingRecordResponse> {
        let response = reqwest::get(self.endpoint(route)).await?;
        let response = response.json::<PendingRecordResponse>().await?;
        Ok(response)
    }

    pub async fn get_package_record(&self, route: &str) -> Result<RecordResponse> {
        let response = reqwest::get(self.endpoint(route)).await?;
        let response = response.json::<RecordResponse>().await?;
        Ok(response)
    }

    pub async fn prove_inclusion(
        &self,
        checkpoint: &MapCheckpoint,
        heads: Vec<LogLeaf>,
    ) -> Result<()> {
        let client = reqwest::Client::new();
        let request = InclusionRequest {
            checkpoint: checkpoint.clone(),
            heads: heads.clone(),
        };
        let response = client
            .post(self.endpoint("/proof/inclusion"))
            .json(&request)
            .send()
            .await?;

        let response = response.json::<InclusionResponse>().await?;

        let log_proof_bundle: LogProofBundle<Sha256, LogLeaf> =
            LogProofBundle::decode(response.log.as_slice())?;
        let (log_data, _, log_inclusions) = log_proof_bundle.unbundle();
        for (leaf, proof) in heads.iter().zip(log_inclusions.iter()) {
            let root = proof.evaluate_value(&log_data, leaf)?;
            if checkpoint.log_root != root.into() {
                return Err(Error::msg("Proof not correct"));
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
            if checkpoint.map_root != root.into() {
                return Err(Error::msg("Proof not correct"));
            }
        }

        Ok(())
    }

    pub async fn prove_log_consistency(&self, old_root: DynHash, new_root: DynHash) -> Result<()> {
        todo!()
    }

    pub async fn upload_content(&self, content: tokio::fs::File) -> Result<()> {
        let client = reqwest::Client::new();
        let _response = client
            .post(self.endpoint("/content/"))
            .body(content)
            .send()
            .await?;
        Ok(())
    }

    pub async fn download_content(&self, digest: DynHash, dir: &Path) -> Result<TempPath> {
        let tmp_path = NamedTempFile::new_in(&dir)?.into_temp_path();
        println!("Downloading {} to {:?}", digest, tmp_path);
        let url_safe = digest.to_string().replace(":", "-");
        let url = self.endpoint(&format!("/content/{}", url_safe));
        let mut stream = reqwest::get(url).await?.bytes_stream();
        let mut file = tokio::fs::File::create(&tmp_path).await?;
        let mut hasher = Sha256::new();
        while let Some(bytes) = stream.next().await.transpose()? {
            hasher.update(bytes.as_ref());
            file.write_all(bytes.as_ref()).await?;
        }
        let hash = hasher.finalize();
        let hash: Hash<Sha256> = hash.into();
        let hash: DynHash = hash.into();
        if hash != digest {
            return Err(Error::msg("Downloaded content digest did not match"));
        }
        Ok(tmp_path)
    }
}
