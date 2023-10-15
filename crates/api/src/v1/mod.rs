//! Types representing v1 of the Warg REST API.

pub mod content;
pub mod fetch;
pub mod package;
pub mod paths;
pub mod proof;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the supported kinds of content sources.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentSource {
    /// The content is located at an GET HTTP URL.
    HttpGet {
        /// The URL of the content.
        url: String,
        /// Optional header names and values for the request.
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        headers: HashMap<String, String>,
        /// Optional support for HTTP Range header.
        #[serde(default, skip_serializing_if = "is_false")]
        supports_range_header: bool,
        /// Optional content size in bytes.
        #[serde(skip_serializing_if = "Option::is_none")]
        size: Option<u64>,
    },
}

fn is_false(b: &bool) -> bool {
    !b
}
