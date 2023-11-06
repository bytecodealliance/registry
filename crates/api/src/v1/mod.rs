//! Types representing v1 of the Warg REST API.

pub mod content;
pub mod fetch;
pub mod ledger;
pub mod monitor;
pub mod package;
pub mod paths;
pub mod proof;

use serde::{Deserialize, Serialize};

/// The HTTP request and response header name that specifies the registry domain whose data is the
/// subject of the request. This header is only expected to be used if referring to a different
/// registry than the host registry.
pub const REGISTRY_HEADER_NAME: &str = "warg-registry";

/// Represents the supported kinds of content sources.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentSource {
    /// The content can be retrieved with an HTTP GET.
    #[serde(rename_all = "camelCase")]
    HttpGet {
        /// The URL of the content.
        url: String,
        /// Optional, server accepts for HTTP Range header.
        /// TODO remove rename, see issue: https://github.com/bytecodealliance/registry/issues/221
        #[serde(
            default,
            skip_serializing_if = "is_false",
            rename = "accept_ranges",
            alias = "acceptRanges"
        )]
        accept_ranges: bool,
        /// Optional, provides content size in bytes.
        #[serde(skip_serializing_if = "Option::is_none")]
        size: Option<u64>,
    },
}

fn is_false(b: &bool) -> bool {
    !b
}
