//! Types relating to the fetch API.

use crate::Status;
use serde::{de::Unexpected, Deserialize, Serialize, Serializer};
use std::{borrow::Cow, collections::HashMap};
use thiserror::Error;
use warg_crypto::hash::AnyHash;
use warg_protocol::{
    registry::{LogId, RecordId, RegistryLen},
    PublishedProtoEnvelopeBody,
};

/// Represents a fetch logs request.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FetchLogsRequest<'a> {
    /// The checkpoint log length.
    pub log_length: RegistryLen,
    /// The limit for the number of operator and package records to fetch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u16>,
    /// The last known operator record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<Cow<'a, RecordId>>,
    /// The map of package identifiers to last known record ids.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub packages: Cow<'a, HashMap<LogId, Option<RecordId>>>,
}

/// Represents a fetch logs response.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchLogsResponse {
    /// Whether there are more records to fetch.
    #[serde(default)]
    pub more: bool,
    /// The operator records appended since the last known operator record.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operator: Vec<PublishedProtoEnvelopeBody>,
    /// The package records appended since last known package record ids.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub packages: HashMap<LogId, Vec<PublishedProtoEnvelopeBody>>,
}

/// Represents a fetch API error.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum FetchError {
    /// The provided checkpoint was not found.
    #[error("checkpoint log length `{0}` was not found")]
    CheckpointNotFound(RegistryLen),
    /// The provided log was not found.
    #[error("log `{0}` was not found")]
    LogNotFound(LogId),
    /// The provided record was not found.
    #[error("record `{0}` was not found")]
    RecordNotFound(RecordId),
    /// An error with a message occurred.
    #[error("{message}")]
    Message {
        /// The HTTP status code.
        status: u16,
        /// The error message
        message: String,
    },
}

impl FetchError {
    /// Returns the HTTP status code of the error.
    pub fn status(&self) -> u16 {
        match self {
            Self::CheckpointNotFound(_) | Self::LogNotFound(_) | Self::RecordNotFound(_) => 404,
            Self::Message { status, .. } => *status,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum EntityType {
    LogLength,
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
    CheckpointNotFound {
        status: Status<404>,
        #[serde(rename = "type")]
        ty: EntityType,
        id: RegistryLen,
    },
    NotFound {
        status: Status<404>,
        #[serde(rename = "type")]
        ty: EntityType,
        id: Cow<'a, T>,
    },
    Message {
        status: u16,
        message: Cow<'a, T>,
    },
}

impl Serialize for FetchError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::CheckpointNotFound(log_length) => RawError::CheckpointNotFound::<RegistryLen> {
                status: Status::<404>,
                ty: EntityType::LogLength,
                id: *log_length,
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
            Self::Message { status, message } => RawError::Message {
                status: *status,
                message: Cow::Borrowed(message),
            }
            .serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for FetchError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawError::<String>::deserialize(deserializer)? {
            RawError::CheckpointNotFound {
                status: _,
                ty: _,
                id,
            } => Ok(Self::CheckpointNotFound(id)),
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
                _ => Err(serde::de::Error::invalid_value(
                    Unexpected::Str(&id),
                    &"a valid log length",
                )),
            },
            RawError::Message { status, message } => Ok(Self::Message {
                status,
                message: message.into_owned(),
            }),
        }
    }
}
