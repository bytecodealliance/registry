use std::sync::Arc;

use anyhow::{Error, Result};

use forrest::{log::LogProofBundle, map::MapProofBundle};
use warg_crypto::hash::Sha256;
use warg_protocol::{
    package,
    registry::{LogLeaf, MapCheckpoint, MapLeaf},
    ProtoEnvelope, SerdeEnvelope,
};
use warg_server::{
    api::{
        fetch::{CheckpointResponse, FetchRequest, FetchResponse},
        package::{PendingRecordResponse, PublishRequest, RecordResponse},
        proof::{InclusionRequest, InclusionResponse},
    },
    services::core::ContentSource,
};

pub struct Client {
    server_url: String,
}

impl Client {
    fn endpoint(&self, route: &str) -> String {
        format!("{}{}", self.server_url, route)
    }

    pub async fn latest_checkpoint(&self) -> Result<Arc<SerdeEnvelope<MapCheckpoint>>> {
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
    ) -> Result<PendingRecordResponse> {
        let client = reqwest::Client::new();
        let request = PublishRequest {
            record: record.as_ref().clone().into(),
            content_sources,
        };
        let url = format!("{}/package/{}", self.server_url, package_name);
        let response = client.post(url).json(&request).send().await?;
        let response = response.json::<PendingRecordResponse>().await?;
        Ok(response)
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
            .post(self.endpoint("/prove/inclusion"))
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

    pub async fn prove_log_consistency() {}

    pub async fn upload_content() {}

    pub async fn download_content() {}
}
