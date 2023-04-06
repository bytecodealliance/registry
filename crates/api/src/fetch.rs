//! Types relating to the fetch API.

use crate::FromError;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use warg_crypto::hash::{DynHash, Hash, Sha256};
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

/// Represents an error from the fetch API.
#[non_exhaustive]
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FetchError {
    /// The provided checkpoint as not found.
    #[error("checkpoint `{checkpoint}` not found")]
    CheckpointNotFound {
        /// The missing checkpoint.
        checkpoint: Hash<Sha256>,
    },
    /// The provided package name was not found.
    #[error("package `{name}` not found")]
    PackageNameNotFound {
        /// The missing package name.
        name: String,
    },
    /// The provided package was not found.
    #[error("package `{id}` not found")]
    PackageNotFound {
        /// The id of the missing package log.
        id: LogId,
    },
    /// The provided package record was not found.
    #[error("package record `{id}` not found")]
    PackageRecordNotFound {
        /// The id of the missing package record.
        id: RecordId,
    },
    /// The provided operator record was not found.
    #[error("operator record `{id}` not found")]
    OperatorRecordNotFound {
        /// The id of the missing operator record.
        id: RecordId,
    },
    /// The provided checkpoint was invalid.
    #[error("invalid checkpoint: {message}")]
    InvalidCheckpoint {
        /// The validation error message.
        message: String,
    },
    /// An error occurred while performing the requested operation.
    #[error("an error occurred while performing the requested operation: {message}")]
    Operation {
        /// The error message.
        message: String,
    },
}

impl From<String> for FetchError {
    fn from(message: String) -> Self {
        Self::Operation { message }
    }
}

impl FromError for FetchError {
    fn from_error<E: std::error::Error>(error: E) -> Self {
        Self::from(error.to_string())
    }
}

/// Represents the result of a fetch API operation.
pub type FetchResult<T> = Result<T, FetchError>;
