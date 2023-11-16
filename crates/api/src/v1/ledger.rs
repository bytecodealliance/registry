//! Types relating to the ledger API.

use serde::{Deserialize, Serialize, Serializer};
use std::borrow::Cow;
use thiserror::Error;
use warg_crypto::hash::HashAlgorithm;
use warg_protocol::registry::RegistryIndex;

/// Represents response a get ledger sources request.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerSourcesResponse {
    /// The hash algorithm used by the ledger.
    pub hash_algorithm: HashAlgorithm,
    /// The list of ledger sources.
    pub sources: Vec<LedgerSource>,
}

/// Ledger source for a specified registry index range. Expected to be sorted in ascending order.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerSource {
    /// First registry index that is included in this ledger source.
    pub first_registry_index: RegistryIndex,
    /// Last registry index that is included in this ledger source.
    pub last_registry_index: RegistryIndex,
    /// The HTTP GET URL location for the ledger source.
    pub url: String,
    /// Content type for the ledger source.
    pub content_type: LedgerSourceContentType,
    /// Optional, server accepts for HTTP Range header.
    #[serde(default, skip_serializing_if = "is_false")]
    pub accept_ranges: bool,
}

fn is_false(b: &bool) -> bool {
    !b
}

/// Content type for the ledger source.
#[derive(Default, PartialEq, Serialize, Deserialize, Debug)]
pub enum LedgerSourceContentType {
    /// The content type is binary representation of the LogId and RecordId hashes without padding.
    /// In the case of `sha256` hash algorithm, this is a repeating sequence of 64 bytes (32 bytes
    /// for each the LogId and RecordId) without padding.
    #[default]
    #[serde(rename = "application/vnd.warg.ledger.packed")]
    Packed,
}

impl LedgerSourceContentType {
    /// Returns the content type represented as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Packed => "application/vnd.warg.ledger.packed",
        }
    }
}

/// Represents a ledger API error.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum LedgerError {
    /// An error with a message occurred.
    #[error("{message}")]
    Message {
        /// The HTTP status code.
        status: u16,
        /// The error message
        message: String,
    },
}

impl LedgerError {
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

impl Serialize for LedgerError {
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

impl<'de> Deserialize<'de> for LedgerError {
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
