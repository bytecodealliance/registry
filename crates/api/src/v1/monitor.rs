//! Types relating to the monitor API.

use crate::Status;
use serde::{de::Unexpected, Deserialize, Serialize, Serializer};
use std::borrow::Cow;
use thiserror::Error;
use warg_crypto::hash::AnyHash;
use warg_crypto::signing;
use warg_protocol::registry::RegistryLen;

/// Represents checkpoint verification response in one of the following states:
#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
#[allow(clippy::large_enum_variant)]
pub enum CheckpointVerificationResponse {
    /// The checkpoint is verified.
    #[serde(rename_all = "camelCase")]
    Verified,
    /// The checkpoint is verified but has not verified the signature.
    #[serde(rename_all = "camelCase")]
    VerifiedButSignatureUnverified,
    /// The checkpoint is unverified but should retry verification.
    #[serde(rename_all = "camelCase")]
    Retry {
        /// Instructs to retry but wait the specified number of seconds.
        wait_seconds: u16,
    },
    /// The checkpoint is unverified and should not retry.
    #[serde(rename_all = "camelCase")]
    Unverified,
}

/// Represents a monitor API error.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum MonitorError {
    /// The provided checkpoint log length is not known to the registry.
    #[error("checkpoint log length `{0}` is not known to the registry")]
    CheckpointNotFound(RegistryLen),
    /// The provided checkpoint log length is greater than the most recent checkpoint. The monitor
    /// is expected to only return this after it is certain that it has the most recent checkpoint.
    #[error("checkpoint log length `{0}` is invalid and greater than the most recent checkpoint")]
    CheckpointInvalidLogLength(RegistryLen),
    /// The checkpoint signature keyId was not found.
    #[error("checkpoint signature keyId `{0}` is unknown")]
    CheckpointSignatureKeyIdNotFound(signing::KeyID),
    /// The checkpoint signature keyId does not currently have permission to sign checkpoints.
    #[error(
        "checkpoint signature keyId `{0}` does not currently have permission to sign checkpoints"
    )]
    CheckpointSignatureKeyIdUnauthorized(signing::KeyID),
    /// The checkpoint signature invalid.
    #[error("checkpoint signature `{0}` is invalid")]
    CheckpointSignatureInvalid(signing::Signature),
    /// The provided checkpoint log root does not match.
    #[error("checkpoint log root `{0}` is invalid")]
    CheckpointLogRootInvalid(AnyHash),
    /// The provided checkpoint map root does not match.
    #[error("checkpoint map root `{0}` is invalid")]
    CheckpointMapRootInvalid(AnyHash),
    /// An error with a message occurred.
    #[error("{message}")]
    Message {
        /// The HTTP status code.
        status: u16,
        /// The error message
        message: String,
    },
}

impl MonitorError {
    /// Returns the HTTP status code of the error.
    pub fn status(&self) -> u16 {
        match self {
            Self::CheckpointNotFound(_) | Self::CheckpointSignatureKeyIdNotFound(_) => 404,
            Self::CheckpointInvalidLogLength(_)
            | Self::CheckpointSignatureKeyIdUnauthorized(_)
            | Self::CheckpointSignatureInvalid(_)
            | Self::CheckpointLogRootInvalid(_)
            | Self::CheckpointMapRootInvalid(_) => 422,
            Self::Message { status, .. } => *status,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum EntityType {
    LogLength,
    KeyId,
    Signature,
    LogRoot,
    MapRoot,
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
    CheckpointInvalid {
        status: Status<422>,
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
    Invalid {
        status: Status<422>,
        #[serde(rename = "type")]
        ty: EntityType,
        id: Cow<'a, T>,
    },
    Message {
        status: u16,
        message: Cow<'a, str>,
    },
}

impl Serialize for MonitorError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::CheckpointNotFound(log_length) => RawError::CheckpointNotFound::<RegistryLen> {
                status: Status::<404>,
                ty: EntityType::LogLength,
                id: *log_length,
            }
            .serialize(serializer),
            Self::CheckpointInvalidLogLength(log_length) => {
                RawError::CheckpointInvalid::<RegistryLen> {
                    status: Status::<422>,
                    ty: EntityType::LogLength,
                    id: *log_length,
                }
                .serialize(serializer)
            }
            Self::CheckpointSignatureKeyIdNotFound(key_id) => RawError::NotFound {
                status: Status::<404>,
                ty: EntityType::KeyId,
                id: Cow::Borrowed(key_id),
            }
            .serialize(serializer),
            Self::CheckpointSignatureKeyIdUnauthorized(key_id) => RawError::Invalid {
                status: Status::<422>,
                ty: EntityType::KeyId,
                id: Cow::Borrowed(key_id),
            }
            .serialize(serializer),
            Self::CheckpointSignatureInvalid(signature) => RawError::Invalid {
                status: Status::<422>,
                ty: EntityType::Signature,
                id: Cow::Borrowed(signature),
            }
            .serialize(serializer),
            Self::CheckpointLogRootInvalid(log_root) => RawError::Invalid {
                status: Status::<422>,
                ty: EntityType::LogRoot,
                id: Cow::Borrowed(log_root),
            }
            .serialize(serializer),
            Self::CheckpointMapRootInvalid(map_root) => RawError::Invalid {
                status: Status::<422>,
                ty: EntityType::MapRoot,
                id: Cow::Borrowed(map_root),
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

impl<'de> Deserialize<'de> for MonitorError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawError::<String>::deserialize(deserializer)? {
            RawError::CheckpointNotFound { id, .. } => Ok(Self::CheckpointNotFound(id)),
            RawError::CheckpointInvalid { id, .. } => Ok(Self::CheckpointInvalidLogLength(id)),
            RawError::NotFound { status: _, ty, id } => match ty {
                EntityType::KeyId => Ok(Self::CheckpointSignatureKeyIdNotFound(
                    signing::KeyID::from(id.into_owned()),
                )),
                _ => Err(serde::de::Error::invalid_value(
                    Unexpected::Str(&id),
                    &"unexpected type",
                )),
            },
            RawError::Invalid { status: _, ty, id } => match ty {
                EntityType::KeyId => Ok(Self::CheckpointSignatureKeyIdUnauthorized(
                    signing::KeyID::from(id.into_owned()),
                )),
                EntityType::Signature => Ok(Self::CheckpointSignatureInvalid(
                    id.parse::<signing::Signature>().map_err(|_| {
                        serde::de::Error::invalid_value(Unexpected::Str(&id), &"a valid signature")
                    })?,
                )),
                EntityType::LogRoot => Ok(Self::CheckpointLogRootInvalid(
                    id.parse::<AnyHash>().map_err(|_| {
                        serde::de::Error::invalid_value(Unexpected::Str(&id), &"a valid log root")
                    })?,
                )),
                EntityType::MapRoot => Ok(Self::CheckpointMapRootInvalid(
                    id.parse::<AnyHash>().map_err(|_| {
                        serde::de::Error::invalid_value(Unexpected::Str(&id), &"a valid map root")
                    })?,
                )),
                _ => Err(serde::de::Error::invalid_value(
                    Unexpected::Str(&id),
                    &"unexpected type",
                )),
            },
            RawError::Message { status, message } => Ok(Self::Message {
                status,
                message: message.into_owned(),
            }),
        }
    }
}
