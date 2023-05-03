//! Types relating to the proof API.

use crate::FromError;
use serde::{Deserialize, Serialize};
use serde_with::{base64::Base64, serde_as};
use thiserror::Error;
use warg_crypto::hash::DynHash;
use warg_protocol::registry::{LogId, LogLeaf, MapCheckpoint};

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

/// Represents an error from the proof API.
#[non_exhaustive]
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ProofError {
    /// The provided log root is invalid.
    #[error("invalid log root: {message}")]
    InvalidLogRoot {
        /// The validation error message.
        message: String,
    },
    /// The provided map root is invalid.
    #[error("invalid map root: {message}")]
    InvalidMapRoot {
        /// The validation error message.
        message: String,
    },
    /// The provided log root was not found.
    #[error("root `{root}` was not found")]
    RootNotFound {
        /// The root that was not found.
        root: DynHash,
    },
    /// The provided log leaf was not found.
    #[error("log leaf `{}:{}` was not found", .leaf.log_id, .leaf.record_id)]
    LeafNotFound {
        /// The leaf that was not found.
        leaf: LogLeaf,
    },
    /// A failure was encountered while bundling proofs.
    #[error("failed to bundle proofs: {message}")]
    BundleFailure {
        /// The failure message.
        message: String,
    },
    /// Failed to prove inclusion of a package.
    #[error("failed to prove inclusion of package `{id}`")]
    PackageNotIncluded {
        /// The id of the package.
        id: LogId,
    },

    /// Failed to prove consistency of a package.
    #[error("failed to prove consistency of `{old_root}` with `{new_root}`")]
    LogNotConsistent {
        /// Old Root
        old_root: DynHash,
        /// New Root
        new_root: DynHash,
    },
    /// The provided root for an inclusion proof was incorrect.
    #[error("failed to prove inclusion: found root `{found}` but was given root `{root}`")]
    IncorrectProof {
        /// The provided root.
        root: DynHash,
        /// The found root.
        found: DynHash,
    },
    /// An error with a message occurred.
    #[error("{message}")]
    Message {
        /// The error message.
        message: String,
    },
}

impl From<String> for ProofError {
    fn from(message: String) -> Self {
        Self::Message { message }
    }
}

impl FromError for ProofError {
    fn from_error<E: std::error::Error>(error: E) -> Self {
        Self::from(error.to_string())
    }
}

/// Represents the result of a proof API operation.
pub type ProofResult<T> = Result<T, ProofError>;
