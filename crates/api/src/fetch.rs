//! Types relating to the fetch API.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use warg_crypto::hash::DynHash;
use warg_protocol::{
    registry::{MapCheckpoint, RecordId},
    ProtoEnvelopeBody, SerdeEnvelope,
};

/// Represents a fetch request.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchRequest {
    /// The root of the registry.
    pub root: DynHash,
    /// The last known operator record.
    pub operator: Option<RecordId>,
    /// The map of packages to last known record ids.
    pub packages: IndexMap<String, Option<RecordId>>,
}

/// Represents a fetch response.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchResponse {
    /// The operator records appended since the last known operator record.
    pub operator: Vec<ProtoEnvelopeBody>,
    /// The package records appended since last known package record ids.
    pub packages: IndexMap<String, Vec<ProtoEnvelopeBody>>,
}

/// Represents a checkpoint response.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointResponse {
    /// The latest registry checkpoint.
    pub checkpoint: SerdeEnvelope<MapCheckpoint>,
}
