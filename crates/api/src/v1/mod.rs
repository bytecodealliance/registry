//! Types representing v1 of the Warg REST API.

pub mod content;
pub mod fetch;
pub mod package;
pub mod paths;
pub mod proof;

use serde::{Deserialize, Serialize};

/// Represents the supported kinds of content sources.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentSource {
    /// The content can be retrieved with an HTTP GET.
    HttpGet {
        /// The URL of the content.
        url: String,
        /// Optional, server accepts for HTTP Range header.
        #[serde(default, skip_serializing_if = "is_false")]
        accept_ranges: bool,
        /// Optional, provides content size in bytes.
        #[serde(skip_serializing_if = "Option::is_none")]
        size: Option<u64>,
    },
}

fn is_false(b: &bool) -> bool {
    !b
}
