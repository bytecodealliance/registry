//! Types relating to the proof API.

use crate::Status;
use serde::{Deserialize, Serialize, Serializer};
use serde_with::{base64::Base64, serde_as};
use std::borrow::Cow;
use thiserror::Error;
use warg_crypto::hash::AnyHash;
use warg_protocol::registry::{LogId};

/// Represents a consistency proof request.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencyRequest {
    /// The starting log length to check for consistency.
    pub from: u32,
    /// The ending log length to check for consistency.
    pub to: u32,
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
    /// The log length to check for inclusion.
    pub log_length: u32,
    /// The log leaf indexes in the registry log to check for inclusion.
    pub leafs: Vec<u32>,
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

/// Represents a proof API error.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ProofError {
    /// The checkpoint could not be found for the provided log length.
    #[error("checkpoint not found for log length {0}")]
    CheckpointNotFound(u32),
    /// The provided log leaf was not found.
    #[error("log leaf `{0}` exceeds the registry log length")]
    LeafNotFound(u32),
    /// Failed to prove inclusion of a package.
    #[error("failed to prove inclusion of package log `{0}`")]
    PackageLogNotIncluded(LogId),
    /// The provided root for an inclusion proof was incorrect.
    #[error("failed to prove inclusion: found root `{found}` but was given root `{root}`")]
    IncorrectProof {
        /// The provided root.
        root: AnyHash,
        /// The found root.
        found: AnyHash,
    },
    /// A failure was encountered while bundling proofs.
    #[error("failed to bundle proofs: {0}")]
    BundleFailure(String),
    /// An error with a message occurred.
    #[error("{message}")]
    Message {
        /// The HTTP status code.
        status: u16,
        /// The error message
        message: String,
    },
}

impl ProofError {
    /// Returns the HTTP status code of the error.
    pub fn status(&self) -> u16 {
        match self {
            Self::CheckpointNotFound(_) | Self::LeafNotFound(_) => 404,
            Self::BundleFailure(_)
            | Self::PackageLogNotIncluded(_)
            | Self::IncorrectProof { .. } => 422,
            Self::Message { status, .. } => *status,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum EntityType {
    LogLength,
    Leaf,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "camelCase")]
enum BundleError<'a> {
    PackageNotIncluded {
        log_id: Cow<'a, LogId>,
    },
    IncorrectProof {
        root: Cow<'a, AnyHash>,
        found: Cow<'a, AnyHash>,
    },
    Failure {
        message: Cow<'a, str>,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
enum RawError<'a>
{
    NotFound {
        status: Status<404>,
        #[serde(rename = "type")]
        ty: EntityType,
        id: u32,
    },
    BundleError {
        status: Status<422>,
        #[serde(flatten)]
        error: BundleError<'a>,
    },
    Message {
        status: u16,
        message: Cow<'a, str>,
    },
}

impl Serialize for ProofError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::CheckpointNotFound(log_length) => RawError::NotFound {
                status: Status::<404>,
                ty: EntityType::LogLength,
                id: *log_length,
            }
            .serialize(serializer),
            Self::LeafNotFound(leaf_index) => RawError::NotFound {
                status: Status::<404>,
                ty: EntityType::Leaf,
                id: *leaf_index,
            }
            .serialize(serializer),
            Self::PackageLogNotIncluded(log_id) => RawError::BundleError {
                status: Status::<422>,
                error: BundleError::PackageNotIncluded {
                    log_id: Cow::Borrowed(log_id),
                },
            }
            .serialize(serializer),
            Self::IncorrectProof { root, found } => RawError::BundleError {
                status: Status::<422>,
                error: BundleError::IncorrectProof {
                    root: Cow::Borrowed(root),
                    found: Cow::Borrowed(found),
                },
            }
            .serialize(serializer),
            Self::BundleFailure(message) => RawError::BundleError {
                status: Status::<422>,
                error: BundleError::Failure {
                    message: Cow::Borrowed(message),
                },
            }
            .serialize(serializer),
            Self::Message { status, message } => RawError::Message {
                status: *status,
                message: Cow::Borrowed(message),
            }
            .serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ProofError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawError::deserialize(deserializer)? {
            RawError::NotFound { status: _, ty, id } => match ty {
                EntityType::LogLength => {
                    Ok(Self::CheckpointNotFound(id))
                }
                EntityType::Leaf => Ok(Self::LeafNotFound(id)),
            },
            RawError::BundleError { status: _, error } => match error {
                BundleError::PackageNotIncluded { log_id } => {
                    Ok(Self::PackageLogNotIncluded(log_id.into_owned()))
                }
                BundleError::IncorrectProof { root, found } => Ok(Self::IncorrectProof {
                    root: root.into_owned(),
                    found: found.into_owned(),
                }),
                BundleError::Failure { message } => Ok(Self::BundleFailure(message.into_owned())),
            },
            RawError::Message { status, message } => Ok(Self::Message {
                status,
                message: message.into_owned(),
            }),
        }
    }
}
