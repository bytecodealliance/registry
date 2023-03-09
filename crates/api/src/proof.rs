//! Types relating to the proof API.

use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use warg_crypto::hash::DynHash;
use warg_protocol::registry::{LogLeaf, MapCheckpoint};

/// Represents a consistency proof request.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencyRequest {
    /// The old root to check for consistency.
    pub old_root: DynHash,
    /// The new root to check for consistency.
    pub new_root: DynHash,
}

/// Represents a consistency proof response.
#[serde_as]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencyResponse {
    /// The bytes of the consistency proof bundle.
    #[serde_as(as = "Base64")]
    pub proof: Vec<u8>,
}

/// Represents an inclusion proof request.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InclusionRequest {
    /// The checkpoint to check for inclusion.
    pub checkpoint: MapCheckpoint,
    /// The heads to check for inclusion.
    pub heads: Vec<LogLeaf>,
}

/// Represents an inclusion proof response.
#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InclusionResponse {
    /// The bytes of the log log proof bundle.
    #[serde_as(as = "Base64")]
    pub log: Vec<u8>,
    /// The bytes of the map inclusion proof bundle.
    #[serde_as(as = "Base64")]
    pub map: Vec<u8>,
}
