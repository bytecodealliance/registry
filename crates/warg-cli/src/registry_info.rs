use serde::{Deserialize, Serialize};
use warg_protocol::{registry::MapCheckpoint, SerdeEnvelope};

#[derive(Serialize, Deserialize)]
pub struct RegistryInfo {
    url: String,
    checkpoint: SerdeEnvelope<MapCheckpoint>,
}

impl RegistryInfo {
    pub fn new(url: String, checkpoint: SerdeEnvelope<MapCheckpoint>) -> Self {
        Self { url, checkpoint }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn checkpoint(&self) -> &SerdeEnvelope<MapCheckpoint> {
        &self.checkpoint
    }
}
