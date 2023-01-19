// https://hackmd.io/1-PXR239TyuEj_d5Eq06Pw?view

// Publish Package Record
// POST /package/<package-id>

// Package Summary
// GET /package/<package-id>

// Package Record
// GET /package/<package-id>/<record-id>

use std::sync::Arc;

use anyhow::{Error, Result};
use axum::extract::State;
use axum::{
    debug_handler, extract::Path, http::StatusCode, response::IntoResponse, routing::post, Json,
    Router,
};
use serde::{Deserialize, Serialize};

use warg_protocol::Envelope;

use crate::services::core::{CoreService, RecordState};
use crate::AnyError;

pub fn build_router(core_service: Arc<CoreService>) -> Router {
    Router::new()
        .route("/package/:package_name", post(publish::publish))
        .with_state(core_service)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename = "lowercase")]
pub enum ContentSource {
    HttpAnonymous { content_digest: String, url: String },
}

#[derive(Serialize, Deserialize)]
pub struct AcceptedRecordResponse {
    record: String,

    content_sources: Vec<ContentSource>,

    checkpoint: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename = "lowercase")]
pub(crate) enum PendingRecordResponse {
    Published { record_url: String },
    Rejected { reason: String },
    Processing,
}

impl PendingRecordResponse {
    pub fn new(state: RecordState) -> Result<Self, AnyError> {
        let response = match state {
            RecordState::Unknown => return Err(Error::msg("Internal error").into()),
            RecordState::Processing => PendingRecordResponse::Processing,
            RecordState::Published { checkpoint } => PendingRecordResponse::Published {
                record_url: "TODO".into(),
            },
            RecordState::Rejected { reason } => PendingRecordResponse::Rejected { reason },
        };
        Ok(response)
    }
}

mod publish {
    use super::*;

    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestBody {
        record: Vec<u8>,
        content_sources: Vec<ContentSource>,
    }

    #[debug_handler]
    pub(crate) async fn publish(
        State(core_service): State<Arc<CoreService>>,
        Path(package_name): Path<String>,
        Json(body): Json<RequestBody>,
    ) -> Result<impl IntoResponse, AnyError> {
        let record = Arc::new(Envelope::from_bytes(body.record)?);

        let state = core_service.new_package_record(package_name, record).await;
        let response = PendingRecordResponse::new(state)?;

        Ok((StatusCode::OK, Json(response)))
    }
}
