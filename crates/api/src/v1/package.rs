//! Types relating to the package API.

use crate::Status;
use serde::{de::Unexpected, Deserialize, Serialize, Serializer};
use std::{borrow::Cow, collections::HashMap};
use thiserror::Error;
use warg_crypto::hash::AnyHash;
use warg_protocol::{
    registry::{LogId, MapCheckpoint, PackageId, RecordId},
    ProtoEnvelopeBody, SerdeEnvelope,
};

/// Represents the supported kinds of content sources.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentSource {
    /// The content is located at an anonymous HTTP URL.
    Http {
        /// The URL of the content.
        url: String,
    },
}

/// Represents a request to publish a record to a package log.
#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct PublishRecordRequest<'a> {
    /// The id of the package being published.
    pub id: Cow<'a, PackageId>,
    /// The publish record to add to the package log.
    pub record: Cow<'a, ProtoEnvelopeBody>,
    /// The complete set of content sources for the record.
    ///
    /// A registry may not support specifying content sources directly.
    pub content_sources: HashMap<AnyHash, Vec<ContentSource>>,
}

/// Represents a package record API entity in a registry.
#[derive(Serialize, Deserialize)]
pub struct PackageRecord {
    /// The identifier of the package record.
    pub id: RecordId,
    /// The current state of the package.
    pub state: PackageRecordState,
}

impl PackageRecord {
    /// Gets the checkpoint of the record.
    ///
    /// Returns `None` if the record hasn't been published yet.
    pub fn checkpoint(&self) -> Option<&SerdeEnvelope<MapCheckpoint>> {
        match &self.state {
            PackageRecordState::Published { checkpoint, .. } => Some(checkpoint),
            _ => None,
        }
    }

    /// Gets the missing content digests of the record.
    pub fn missing_content(&self) -> &[AnyHash] {
        match &self.state {
            PackageRecordState::Sourcing {
                missing_content, ..
            } => missing_content,
            _ => &[],
        }
    }
}

/// Represents a package record in one of the following states:
/// * `sourcing` - The record is sourcing content.
/// * `processing` - The record is being processed.
/// * `rejected` - The record was rejected.
/// * `published` - The record was published to the log.
#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum PackageRecordState {
    /// The package record needs content sources.
    Sourcing {
        /// The digests of the missing content.
        missing_content: Vec<AnyHash>,
    },
    /// The package record is processing.
    Processing,
    /// The package record is rejected.
    Rejected {
        /// The reason the record was rejected.
        reason: String,
    },
    /// The package record was successfully published to the log.
    Published {
        /// The checkpoint that the recorded was included in.
        checkpoint: SerdeEnvelope<MapCheckpoint>,
        /// The envelope of the package record.
        record: ProtoEnvelopeBody,
        /// The content sources of the record.
        content_sources: HashMap<AnyHash, Vec<ContentSource>>,
    },
}

/// Represents a package API error.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum PackageError {
    /// The provided log was not found.
    #[error("log `{0}` was not found")]
    LogNotFound(LogId),
    /// The provided record was not found.
    #[error("record `{0}` was not found")]
    RecordNotFound(RecordId),
    /// The record is not currently sourcing content.
    #[error("the record is not currently sourcing content")]
    RecordNotSourcing,
    /// The operation was not authorized by the registry.
    #[error("unauthorized operation: {0}")]
    Unauthorized(String),
    /// The operation was not supported by the registry.
    #[error("the requested operation is not supported: {0}")]
    NotSupported(String),
    /// The package was rejected by the registry.
    #[error("the package was rejected by the registry: {0}")]
    Rejection(String),
    /// An error with a message occurred.
    #[error("{message}")]
    Message {
        /// The HTTP status code.
        status: u16,
        /// The error message
        message: String,
    },
}

impl PackageError {
    /// Returns the HTTP status code of the error.
    pub fn status(&self) -> u16 {
        match self {
            // Note: this is 403 and not a 401 as the registry does not use
            // HTTP authentication.
            Self::Unauthorized { .. } => 403,
            Self::LogNotFound(_) | Self::RecordNotFound(_) => 404,
            Self::RecordNotSourcing => 405,
            Self::Rejection(_) => 422,
            Self::NotSupported(_) => 501,
            Self::Message { status, .. } => *status,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum EntityType {
    Log,
    Record,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
enum RawError<'a, T>
where
    T: Clone + ToOwned,
    <T as ToOwned>::Owned: Serialize + for<'b> Deserialize<'b>,
{
    Unauthorized {
        status: Status<403>,
        message: Cow<'a, str>,
    },
    NotFound {
        status: Status<404>,
        #[serde(rename = "type")]
        ty: EntityType,
        id: Cow<'a, T>,
    },
    RecordNotSourcing {
        status: Status<405>,
    },
    Rejection {
        status: Status<422>,
        message: Cow<'a, str>,
    },
    NotSupported {
        status: Status<501>,
        message: Cow<'a, str>,
    },
    Message {
        status: u16,
        message: Cow<'a, str>,
    },
}

impl Serialize for PackageError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Unauthorized(message) => RawError::Unauthorized::<()> {
                status: Status::<403>,
                message: Cow::Borrowed(message),
            }
            .serialize(serializer),
            Self::LogNotFound(log_id) => RawError::NotFound {
                status: Status::<404>,
                ty: EntityType::Log,
                id: Cow::Borrowed(log_id),
            }
            .serialize(serializer),
            Self::RecordNotFound(record_id) => RawError::NotFound {
                status: Status::<404>,
                ty: EntityType::Record,
                id: Cow::Borrowed(record_id),
            }
            .serialize(serializer),
            Self::RecordNotSourcing => RawError::RecordNotSourcing::<()> {
                status: Status::<405>,
            }
            .serialize(serializer),
            Self::Rejection(message) => RawError::Rejection::<()> {
                status: Status::<422>,
                message: Cow::Borrowed(message),
            }
            .serialize(serializer),
            Self::NotSupported(message) => RawError::NotSupported::<()> {
                status: Status::<501>,
                message: Cow::Borrowed(message),
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

impl<'de> Deserialize<'de> for PackageError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawError::<String>::deserialize(deserializer)? {
            RawError::Unauthorized { status: _, message } => {
                Ok(Self::Unauthorized(message.into_owned()))
            }
            RawError::NotFound { status: _, ty, id } => match ty {
                EntityType::Log => Ok(Self::LogNotFound(
                    id.parse::<AnyHash>()
                        .map_err(|_| {
                            serde::de::Error::invalid_value(Unexpected::Str(&id), &"a valid log id")
                        })?
                        .into(),
                )),
                EntityType::Record => Ok(Self::RecordNotFound(
                    id.parse::<AnyHash>()
                        .map_err(|_| {
                            serde::de::Error::invalid_value(
                                Unexpected::Str(&id),
                                &"a valid record id",
                            )
                        })?
                        .into(),
                )),
            },
            RawError::RecordNotSourcing { status: _ } => Ok(Self::RecordNotSourcing),
            RawError::Rejection { status: _, message } => Ok(Self::Rejection(message.into_owned())),
            RawError::NotSupported { status: _, message } => {
                Ok(Self::NotSupported(message.into_owned()))
            }
            RawError::Message { status, message } => Ok(Self::Message {
                status,
                message: message.into_owned(),
            }),
        }
    }
}
