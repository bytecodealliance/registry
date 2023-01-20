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
use serde::{Deserialize, Serialize};

use warg_crypto::hash::{DynHash, Sha256};
use warg_protocol::registry::RecordId;
use warg_protocol::Envelope;

use crate::services::core::{ContentSource, CoreService, RecordState};
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
            .route("/:package_name", post(publish::publish))
            .route("/:package_name/records/:record_id", get(get_record::get_record))
            .route("/:package_name/pending/:record_id", get(get_pending_record::get_pending_record))
            .with_state(self)
    }
}

pub fn record_url(package_name: String, record_id: RecordId) -> String {
    format!("/{package_name}/{record_id}")
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename = "lowercase")]
pub(crate) enum PendingRecordResponse {
    Published { record_url: String },
    Rejected { reason: String },
    Processing,
}

impl PendingRecordResponse {
    pub fn new(
        package_name: String,
        record_id: RecordId,
        state: RecordState,
    ) -> Result<Self, AnyError> {
        let response = match state {
            RecordState::Unknown => return Err(Error::msg("Internal error").into()),
            RecordState::Processing => PendingRecordResponse::Processing,
            RecordState::Published { .. } => PendingRecordResponse::Published {
                record_url: record_url(package_name, record_id),
            },
            RecordState::Rejected { reason } => PendingRecordResponse::Rejected { reason },
        };
        Ok(response)
    }
}

mod publish {
    use reqwest::Client;

    use crate::services::core::ContentSourceKind;

    use super::*;

    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestBody {
        record: Vec<u8>,
        content_sources: Vec<ContentSource>,
    }

    #[debug_handler]
    pub(crate) async fn publish(
        State(config): State<Config>,
        Path(package_name): Path<String>,
        Json(body): Json<RequestBody>,
    ) -> Result<impl IntoResponse, AnyError> {
        let record = Arc::new(Envelope::from_bytes(body.record)?);

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
}

mod get_record {
    use warg_protocol::{package::PackageRecord, registry::MapCheckpoint};

    use crate::services::core::PackageRecordInfo;

    use super::*;

    #[derive(Serialize)]
    pub struct ResponseBody {
        record: Arc<Envelope<PackageRecord>>,
        content_sources: Arc<Vec<ContentSource>>,
        checkpoint: Arc<Envelope<MapCheckpoint>>,
    }

    #[debug_handler]
    pub(crate) async fn get_record(
        State(config): State<Config>,
        Path((package_name, record_id)): Path<(String, String)>,
    ) -> Result<impl IntoResponse, AnyError> {
        let record_id: DynHash = record_id.parse()?;
        let record_id: RecordId = record_id.into();

        let info = config.core_service.get_package_record_info(package_name, record_id).await;
        match info {
            Some(PackageRecordInfo { record, content_sources, state: RecordState::Published { checkpoint }}) => {
                let response = ResponseBody {
                    record,
                    content_sources,
                    checkpoint
                };
                Ok((StatusCode::OK, Json(response)))
            },
            _ => Err(Error::msg("Not found").into()) // todo: improve to 404
        }
    }
}

mod get_pending_record {
    use super::*;

    #[debug_handler]
    pub(crate) async fn get_pending_record(
        State(config): State<Config>,
        Path((package_name, record_id)): Path<(String, String)>,
    ) -> Result<impl IntoResponse, AnyError> {
        let record_id: DynHash = record_id.parse()?;
        let record_id: RecordId = record_id.into();

        let status = config.core_service.get_package_record_status(package_name, record_id).await;
        Ok((StatusCode::OK, Json(status)))
    }
}
