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
    pub signature: CheckpointVerificationState,
    /// Optional, retry after specified number of seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u16>,
}

/// Represents checkpoint verification state.
#[derive(Eq, PartialEq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckpointVerificationState {
    /// The checkpoint is unverified and could be valid or invalid.
    #[serde(rename_all = "camelCase")]
    Unverified,
    /// The checkpoint is verified.
    #[serde(rename_all = "camelCase")]
    Verified,
    /// The checkpoint is invalid.
    #[serde(rename_all = "camelCase")]
    Invalid,
}

/// Represents a monitor API error.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum MonitorError {
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
            Self::Message { status, .. } => *status,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged, rename_all = "camelCase")]
enum RawError<'a> {
    Message { status: u16, message: Cow<'a, str> },
}

impl Serialize for MonitorError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
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
            RawError::Message { status, message } => Ok(Self::Message {
                status,
                message: message.into_owned(),
            }),
        }
    }
}
