//! Types relating to the monitor API.

use serde::{Deserialize, Serialize, Serializer};
use std::borrow::Cow;
use thiserror::Error;

/// Represents checkpoint verification response.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointVerificationResponse {
    /// The checkpoint verification state.
    pub checkpoint: CheckpointVerificationState,
    /// The checkpoint signature verification state.
    pub signature: CheckpointSignatureVerificationState,
}

/// Represents checkpoint verification state.
#[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckpointVerificationState {
    /// The checkpoint signature is unverified and could be valid or invalid.
    #[serde(rename_all = "camelCase")]
    Unverified,
    /// The checkpoint is verified.
    #[serde(rename_all = "camelCase")]
    Verified,
    /// The checkpoint log length does not exist.
    #[serde(rename_all = "camelCase")]
    NotFound,
    /// The checkpoint is invalid.
    #[serde(rename_all = "camelCase")]
    Invalid,
}

/// Represents checkpoint signature verification state.
#[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckpointSignatureVerificationState {
    /// The checkpoint signature is unverified and could be valid or invalid.
    #[serde(rename_all = "camelCase")]
    Unverified,
    /// The checkpoint signature is verified.
    #[serde(rename_all = "camelCase")]
    Verified,
    /// The checkpoint signature key ID is known but not authorized to sign checkpoints.
    #[serde(rename_all = "camelCase")]
    Unauthorized,
    /// The checkpoint signature is not valid. The key ID could be not known or the signature is incorrect.
    #[serde(rename_all = "camelCase")]
    Invalid,
}

/// Represents a monitor API error.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum MonitorError {
    /// Instruct to retry after specified number of seconds.
    #[error("retry after {0} seconds")]
    RetryAfter(u16),
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
            Self::RetryAfter(_) => 503,
            Self::Message { status, .. } => *status,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
enum RawError<'a> {
    #[serde(rename_all = "camelCase")]
    RetryAfter {
        status: u16,
        retry_after: u16,
    },
    Message {
        status: u16,
        message: Cow<'a, str>,
    },
}

impl Serialize for MonitorError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::RetryAfter(seconds) => RawError::RetryAfter {
                status: 503,
                retry_after: *seconds,
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

impl<'de> Deserialize<'de> for MonitorError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawError::deserialize(deserializer)? {
            RawError::RetryAfter { retry_after, .. } => Ok(Self::RetryAfter(retry_after)),
            RawError::Message { status, message } => Ok(Self::Message {
                status,
                message: message.into_owned(),
            }),
        }
    }
}
