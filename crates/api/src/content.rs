//! Types relating to the content API.

use serde::{Deserialize, Serialize};
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
      /// The URL for the content
      url: String 
    },
}
