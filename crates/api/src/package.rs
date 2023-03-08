//! Types relating to the package API.

use crate::content::ContentSource;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warg_protocol::{registry::MapCheckpoint, ProtoEnvelopeBody, SerdeEnvelope};

/// Represents a request to publish a package.
#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct PublishRequest {
    /// The publish record to add to the package log.
    pub record: ProtoEnvelopeBody,
    /// The content sources for the record.
    pub content_sources: Vec<ContentSource>,
}

/// Represents a pending record response.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "state", rename = "camelCase")]
pub enum PendingRecordResponse {
    /// The record has been published.
    Published {
        /// The URL of the published record.
        record_url: String,
    },
    /// The record has been rejected.
    Rejected {
        /// The reason the record was rejected.
        reason: String,
    },
    /// The record is still being processed.
    Processing {
        /// The URL of the publishing status.
        status_url: String,
    },
}

/// Represents a response to a record request.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RecordResponse {
    /// The body of the record.
    pub record: ProtoEnvelopeBody,
    /// The content sources of the record.
    pub content_sources: Arc<Vec<ContentSource>>,
    /// The checkpoint of the record.
    pub checkpoint: Arc<SerdeEnvelope<MapCheckpoint>>,
}
