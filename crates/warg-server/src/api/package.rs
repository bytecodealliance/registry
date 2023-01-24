use std::sync::Arc;

use anyhow::{Error, Result};
use axum::extract::State;
use axum::{
    debug_handler,
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use warg_crypto::hash::{DynHash, Sha256};
use warg_protocol::registry::RecordId;
use warg_protocol::ProtoEnvelopeBody;
use warg_protocol::{registry::MapCheckpoint, SerdeEnvelope};

use crate::services::core::PackageRecordInfo;
use crate::services::core::{ContentSource, ContentSourceKind, CoreService, RecordState};
use crate::AnyError;

#[derive(Clone)]
pub struct Config {
    core_service: Arc<CoreService>,
    base_url: String,
}

impl Config {
    pub fn new(core_service: Arc<CoreService>, base_url: String) -> Self {
        Self {
            core_service,
            base_url,
        }
    }

    pub fn build_router(self) -> Router {
        Router::new()
            .route("/:package_name", post(publish))
            .route("/:package_name/records/:record_id", get(get_record))
            .route("/:package_name/pending/:record_id", get(get_pending_record))
            .with_state(self)
    }
}

fn record_url(package_name: String, record_id: RecordId) -> String {
    format!("/package/{package_name}/records/{record_id}")
}

fn pending_record_url(package_name: String, record_id: RecordId) -> String {
    format!("/package/{package_name}/pending/{record_id}")
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename = "lowercase")]
pub enum PendingRecordResponse {
    Published { record_url: String },
    Rejected { reason: String },
    Processing { status_url: String },
}

impl PendingRecordResponse {
    pub(crate) fn new(
        package_name: String,
        record_id: RecordId,
        state: RecordState,
    ) -> Result<Self, AnyError> {
        let response = match state {
            RecordState::Unknown => return Err(Error::msg("Internal error").into()),
            RecordState::Processing => PendingRecordResponse::Processing {
                status_url: pending_record_url(package_name, record_id),
            },
            RecordState::Published { .. } => PendingRecordResponse::Published {
                record_url: record_url(package_name, record_id),
            },
            RecordState::Rejected { reason } => PendingRecordResponse::Rejected { reason },
        };
        Ok(response)
    }
}

#[derive(Serialize, Deserialize)]
pub struct PublishRequest {
    pub record: ProtoEnvelopeBody,
    pub content_sources: Vec<ContentSource>,
}

#[debug_handler]
pub(crate) async fn publish(
    State(config): State<Config>,
    Path(package_name): Path<String>,
    Json(body): Json<PublishRequest>,
) -> Result<impl IntoResponse, AnyError> {
    let record = Arc::new(body.record.try_into()?);

    let record_id = RecordId::package_record::<Sha256>(&record);

    for source in body.content_sources.iter() {
        match &source.kind {
            ContentSourceKind::HttpAnonymous { url } => {
                if url.starts_with(&config.base_url) {
                    let response = Client::builder().build()?.head(url).send().await?;
                    if !response.status().is_success() {
                        return Err(Error::msg("Unable to validate content is present").into());
                    }
                } else {
                    return Err(Error::msg("URL not from current host").into());
                }
            }
        }
    }

    let state = config
        .core_service
        .submit_package_record(package_name.clone(), record, body.content_sources)
        .await;
    let response = PendingRecordResponse::new(package_name.clone(), record_id, state)?;

    Ok((StatusCode::OK, Json(response)))
}

#[derive(Serialize, Deserialize)]
pub struct RecordResponse {
    record: ProtoEnvelopeBody,
    content_sources: Arc<Vec<ContentSource>>,
    checkpoint: Arc<SerdeEnvelope<MapCheckpoint>>,
}

#[debug_handler]
pub(crate) async fn get_record(
    State(config): State<Config>,
    Path((package_name, record_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, AnyError> {
    let record_id: DynHash = record_id.parse()?;
    let record_id: RecordId = record_id.into();

    let info = config
        .core_service
        .get_package_record_info(package_name, record_id)
        .await;
    match info {
        Some(PackageRecordInfo {
            record,
            content_sources,
            state: RecordState::Published { checkpoint },
        }) => {
            let response = RecordResponse {
                record: record.as_ref().clone().into(),
                content_sources,
                checkpoint: checkpoint,
            };
            Ok((StatusCode::OK, Json(response)))
        }
        _ => Err(Error::msg("Not found").into()), // todo: improve to 404
    }
}

#[debug_handler]
pub(crate) async fn get_pending_record(
    State(config): State<Config>,
    Path((package_name, record_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, AnyError> {
    let record_id: DynHash = record_id.parse()?;
    let record_id: RecordId = record_id.into();

    let status = config
        .core_service
        .get_package_record_status(&package_name, record_id.clone())
        .await;

    let response = PendingRecordResponse::new(package_name, record_id, status)?;

    Ok((StatusCode::OK, Json(response)))
}
