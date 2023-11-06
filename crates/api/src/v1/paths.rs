//! The paths of the Warg REST API.

use warg_crypto::hash::AnyHash;
use warg_protocol::registry::{LogId, RecordId};

/// The path of the "fetch logs" API.
pub fn fetch_logs() -> &'static str {
    "v1/fetch/logs"
}

/// The path of the "fetch checkpoint" API.
pub fn fetch_checkpoint() -> &'static str {
    "v1/fetch/checkpoint"
}

/// The path of the "fetch package IDs" API.
pub fn fetch_package_ids() -> &'static str {
    "v1/fetch/ids"
}

/// The path of the get ledger sources.
pub fn ledger_sources() -> &'static str {
    "v1/ledger"
}

/// The path of the "publish package record" API.
pub fn publish_package_record(log_id: &LogId) -> String {
    format!("v1/package/{log_id}/record")
}

/// The path to request download of content digest.
pub fn content_sources(digest: &AnyHash) -> String {
    format!("v1/content/{digest}")
}

/// The path for a package record.
pub fn package_record(log_id: &LogId, record_id: &RecordId) -> String {
    format!("v1/package/{log_id}/record/{record_id}")
}

/// The path for proving checkpoint consistency.
pub fn prove_consistency() -> &'static str {
    "v1/proof/consistency"
}

/// The path for proving checkpoint inclusion.
pub fn prove_inclusion() -> &'static str {
    "v1/proof/inclusion"
}

/// The path for verifying a checkpoint.
pub fn verify_checkpoint() -> &'static str {
    "v1/verify/checkpoint"
}
