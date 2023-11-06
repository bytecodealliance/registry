//! Types relating to the fetch API.

use crate::Status;
use serde::{de::Unexpected, Deserialize, Serialize, Serializer};
use std::{borrow::Cow, collections::HashMap};
use thiserror::Error;
use warg_crypto::hash::AnyHash;
use warg_protocol::{
    registry::{LogId, PackageId, RegistryLen},
    PublishedProtoEnvelopeBody,
};

/// Wraps the PublishedProtoEnvelopeBody with a fetch token.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedRecord {
    /// Record proto envelope body with RegistryIndex.
    #[serde(flatten)]
    pub envelope: PublishedProtoEnvelopeBody,
    /// Fetch token for fetch pagination.
    pub fetch_token: String,
}

/// Represents a fetch logs request.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FetchLogsRequest<'a> {
    /// The checkpoint log length.
    pub log_length: RegistryLen,
    /// The limit for the number of operator and package records to fetch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u16>,
    /// The last known operator record fetch token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<Cow<'a, str>>,
    /// The map of package identifiers to last known fetch token.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub packages: Cow<'a, HashMap<LogId, Option<String>>>,
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
    pub operator: Vec<PublishedRecord>,
    /// The package records appended since last known package record ids.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub packages: HashMap<LogId, Vec<PublishedRecord>>,
}

/// Represents a fetch package IDs request.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchPackageIdsRequest<'a> {
    /// List of package log IDs to request the package name.
    pub packages: Cow<'a, Vec<LogId>>,
}

/// Represents a fetch package IDs response. If the requested number of packages exceeds the limit
/// that the server can fulfill on a single request, the client should retry with the log IDs that
/// are absent in the response body.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchPackageIdsResponse {
    /// The log ID hash mapping to a package ID. If `None`, the package ID cannot be provided.
    pub packages: HashMap<LogId, Option<PackageId>>,
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
    /// The provided fetch token was not found.
    #[error("fetch token `{0}` was not found")]
    FetchTokenNotFound(String),
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
            Self::CheckpointNotFound(_) | Self::LogNotFound(_) | Self::FetchTokenNotFound(_) => 404,
            Self::Message { status, .. } => *status,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum EntityType {
    LogLength,
    Log,
    FetchToken,
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
            Self::FetchTokenNotFound(token) => RawError::NotFound {
                status: Status::<404>,
                ty: EntityType::FetchToken,
                id: Cow::Borrowed(token),
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
            RawError::CheckpointNotFound { id, .. } => Ok(Self::CheckpointNotFound(id)),
            RawError::NotFound { status: _, ty, id } => match ty {
                EntityType::Log => Ok(Self::LogNotFound(
                    id.parse::<AnyHash>()
                        .map_err(|_| {
                            serde::de::Error::invalid_value(Unexpected::Str(&id), &"a valid log id")
                        })?
                        .into(),
                )),
                EntityType::FetchToken => Ok(Self::FetchTokenNotFound(id.into_owned())),
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
