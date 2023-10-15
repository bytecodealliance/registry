//! Types relating to the content API.

pub use super::ContentSource;
use crate::Status;
use serde::{de::Unexpected, Deserialize, Serialize, Serializer};
use std::{borrow::Cow, collections::HashMap};
use thiserror::Error;
use warg_crypto::hash::AnyHash;

/// Represents a response for content digest.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSourcesResponse {
    /// The content sources for the requested content digest, as well as additional
    /// content digests imported by the requested content digest.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub content_sources: HashMap<AnyHash, Vec<ContentSource>>,
}

/// Represents a content API error.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ContentError {
    /// The provided content digest was not found.
    #[error("content digest `{0}` was not found")]
    ContentDigestNotFound(AnyHash),
    /// An error with a message occurred.
    #[error("{message}")]
    Message {
        /// The HTTP status code.
        status: u16,
        /// The error message
        message: String,
    },
}

impl ContentError {
    /// Returns the HTTP status code of the error.
    pub fn status(&self) -> u16 {
        match self {
            Self::ContentDigestNotFound(_) => 404,
            Self::Message { status, .. } => *status,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum EntityType {
    ContentDigest,
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
    Message {
        status: u16,
        message: Cow<'a, str>,
    },
}

impl Serialize for ContentError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::ContentDigestNotFound(digest) => RawError::NotFound {
                status: Status::<404>,
                ty: EntityType::ContentDigest,
                id: Cow::Borrowed(digest),
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

impl<'de> Deserialize<'de> for ContentError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawError::<String>::deserialize(deserializer)? {
            RawError::NotFound { status: _, ty, id } => match ty {
                EntityType::ContentDigest => Ok(Self::ContentDigestNotFound(
                    id.parse::<AnyHash>().map_err(|_| {
                        serde::de::Error::invalid_value(Unexpected::Str(&id), &"a valid digest")
                    })?,
                )),
            },
            RawError::Message { status, message } => Ok(Self::Message {
                status,
                message: message.into_owned(),
            }),
        }
    }
}
