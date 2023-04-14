//! Types relating to the content API.

use crate::FromError;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use warg_crypto::hash::DynHash;

/// Represents a content source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSource {
    /// The digest of the content.
    pub digest: DynHash,
    /// The kind of content source.
    pub kind: ContentSourceKind,
}

/// Represents the supported kinds of content sources.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentSourceKind {
    /// The content is located at an anonymous HTTP URL.
    HttpAnonymous {
        /// The URL of the content.
        url: String,
    },
}

/// Represents an error from the content API.
#[non_exhaustive]
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentError {
    /// The service failed to allocate temporary file storage.
    #[error("failed to allocate temporary file storage")]
    TempFile,
    /// The service failed to read the request body.
    #[error("failed to read request body: {message}")]
    BodyRead {
        /// The error message.
        message: String,
    },
    /// The service failed to write to temporary file storage.
    #[error("an error occurred while writing to temporary file storage: {message}")]
    IoError {
        /// The error message.
        message: String,
    },
    /// The service failed to persist the temporary file to the content directory.
    #[error("failed to persist temporary file to content directory")]
    FailedToPersist,
    /// An error with a message occurred.
    #[error("{message}")]
    Message {
        /// The error message.
        message: String,
    },
}

impl From<String> for ContentError {
    fn from(message: String) -> Self {
        Self::Message { message }
    }
}

impl FromError for ContentError {
    fn from_error<E: std::error::Error>(error: E) -> Self {
        Self::from(error.to_string())
    }
}

/// Represents the result of a content API operation.
pub type ContentResult<T> = Result<T, ContentError>;
