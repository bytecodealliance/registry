//! Types relating to the fetch API.

use crate::FromError;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use warg_crypto::hash::DynHash;
use warg_protocol::{
    registry::{LogId, MapCheckpoint, RecordId},
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
    #[serde(default)]
    pub packages: IndexMap<String, Option<RecordId>>,
}

/// Represents a fetch response.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchResponse {
    /// The operator records appended since the last known operator record.
    pub operator: Vec<ProtoEnvelopeBody>,
    /// The package records appended since last known package record ids.
    #[serde(default)]
    pub packages: IndexMap<String, Vec<ProtoEnvelopeBody>>,
}

/// Represents a checkpoint response.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointResponse {
    /// The latest registry checkpoint.
    pub checkpoint: SerdeEnvelope<MapCheckpoint>,
}

/// Represents an error from the fetch API.
#[non_exhaustive]
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FetchError {
    /// The provided checkpoint was not found.
    #[error("checkpoint `{checkpoint}` was not found")]
    CheckpointNotFound {
        /// The missing checkpoint.
        checkpoint: DynHash,
    },
    /// The log was not found.
    #[error("log `{log_id}` was not found")]
    LogNotFound {
        /// The missing operator log id.
        log_id: LogId,
    },
    /// The provided package name was not found.
    #[error("package `{name}` was not found")]
    PackageNotFound {
        /// The missing package name.
        name: String,
    },
    /// The provided record was not found.
    #[error("record `{record_id}` was not found")]
    RecordNotFound {
        /// The id of the missing record.
        record_id: RecordId,
    },
    /// An error occurred while performing the requested operation.
    #[error("an error occurred while performing the requested operation")]
    Operation,
    /// An error with a message occurred.
    #[error("{message}")]
    Message {
        /// The error message.
        message: String,
    },
}

impl From<String> for FetchError {
    fn from(message: String) -> Self {
        Self::Message { message }
    }
}

impl FromError for FetchError {
    fn from_error<E: std::error::Error>(error: E) -> Self {
        Self::from(error.to_string())
    }
}

/// Represents the result of a fetch API operation.
pub type FetchResult<T> = Result<T, FetchError>;
