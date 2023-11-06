//! Types relating to the ledger API.

use serde::{Deserialize, Serialize};
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
    /// Optional, server accepts for HTTP Range header.
    #[serde(skip_serializing_if = "is_false")]
    pub accept_ranges: bool,
    /// Content type for the ledger source.
    #[serde(default, skip_serializing_if = "is_ledger_packed")]
    pub content_type: LedgerSourceContentType,
}

fn is_false(b: &bool) -> bool {
    !b
}

fn is_ledger_packed(content_type: &LedgerSourceContentType) -> bool {
    content_type == &LedgerSourceContentType::Packed
}

/// Content type for the ledger source.
#[derive(Default, PartialEq, Serialize, Deserialize)]
pub enum LedgerSourceContentType {
    /// The content type is binary representation of the LogId and RecordId hashes without padding.
    /// In the case of `sha256` hash algorithm, this is a repeating sequence of 64 bytes (32 bytes
    /// for each the LogId and RecordId) without padding.
    #[default]
    #[serde(rename = "application/vnd.bytecodealliance.registry.ledger.packed")]
    Packed,
}

impl LedgerSourceContentType {
    /// Returns the content type represented as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Packed => "application/vnd.bytecodealliance.registry.ledger.packed",
        }
    }
}
