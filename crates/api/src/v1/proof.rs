//! Types relating to the proof API.

use crate::Status;
use serde::{de::Unexpected, Deserialize, Serialize, Serializer};
use serde_with::{base64::Base64, serde_as};
use std::borrow::Cow;
use thiserror::Error;
use warg_crypto::hash::AnyHash;
use warg_protocol::registry::{Checkpoint, LogId, LogLeaf};

/// Represents a consistency proof request.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsistencyRequest<'a> {
    /// The starting log root hash to check for consistency.
    pub from: Cow<'a, AnyHash>,
    /// The ending log root hash to check for consistency.
    pub to: Cow<'a, AnyHash>,
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
pub struct InclusionRequest<'a> {
    /// The checkpoint to check for inclusion.
    pub checkpoint: Cow<'a, Checkpoint>,
    /// The log leafs to check for inclusion.
    pub leafs: Cow<'a, [LogLeaf]>,
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
    /// The provided log root was not found.
    #[error("log root `{0}` was not found")]
    RootNotFound(AnyHash),
    /// The provided log leaf was not found.
    #[error("log leaf `{}:{}` was not found", .0.log_id, .0.record_id)]
    LeafNotFound(LogLeaf),
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
            Self::RootNotFound(_) | Self::LeafNotFound(_) => 404,
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
    LogRoot,
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
enum RawError<'a, T>
where
    T: Clone + ToOwned,
    <T as ToOwned>::Owned: Serialize + for<'b> Deserialize<'b>,
{
    NotFound {
        status: Status<404>,
        #[serde(rename = "type")]
        ty: EntityType,
        id: Cow<'a, T>,
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
            Self::RootNotFound(root) => RawError::NotFound {
                status: Status::<404>,
                ty: EntityType::LogRoot,
                id: Cow::Borrowed(root),
            }
            .serialize(serializer),
            Self::LeafNotFound(leaf) => RawError::NotFound::<String> {
                status: Status::<404>,
                ty: EntityType::Leaf,
                id: Cow::Owned(format!("{}|{}", leaf.log_id, leaf.record_id)),
            }
            .serialize(serializer),
            Self::PackageLogNotIncluded(log_id) => RawError::BundleError::<()> {
                status: Status::<422>,
                error: BundleError::PackageNotIncluded {
                    log_id: Cow::Borrowed(log_id),
                },
            }
            .serialize(serializer),
            Self::IncorrectProof { root, found } => RawError::BundleError::<()> {
                status: Status::<422>,
                error: BundleError::IncorrectProof {
                    root: Cow::Borrowed(root),
                    found: Cow::Borrowed(found),
                },
            }
            .serialize(serializer),
            Self::BundleFailure(message) => RawError::BundleError::<()> {
                status: Status::<422>,
                error: BundleError::Failure {
                    message: Cow::Borrowed(message),
                },
            }
            .serialize(serializer),
            Self::Message { status, message } => RawError::Message::<()> {
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
        match RawError::<String>::deserialize(deserializer)? {
            RawError::NotFound { status: _, ty, id } => match ty {
                EntityType::LogRoot => {
                    Ok(Self::RootNotFound(id.parse::<AnyHash>().map_err(|_| {
                        serde::de::Error::invalid_value(
                            Unexpected::Str(&id),
                            &"a valid checkpoint id",
                        )
                    })?))
                }
                EntityType::Leaf => Ok(Self::LeafNotFound(
                    id.split_once('|')
                        .map(|(log_id, record_id)| {
                            Ok(LogLeaf {
                                log_id: log_id
                                    .parse::<AnyHash>()
                                    .map_err(|_| {
                                        serde::de::Error::invalid_value(
                                            Unexpected::Str(log_id),
                                            &"a valid log id",
                                        )
                                    })?
                                    .into(),
                                record_id: record_id
                                    .parse::<AnyHash>()
                                    .map_err(|_| {
                                        serde::de::Error::invalid_value(
                                            Unexpected::Str(record_id),
                                            &"a valid record id",
                                        )
                                    })?
                                    .into(),
                            })
                        })
                        .ok_or_else(|| {
                            serde::de::Error::invalid_value(
                                Unexpected::Str(&id),
                                &"a valid leaf id",
                            )
                        })??,
                )),
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
