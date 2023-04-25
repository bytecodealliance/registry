use crate::{
    datastore::{PackageLogEntry, RecordStatus},
    services::{CoreService, CoreServiceError},
};
use anyhow::Error;
use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use reqwest::Client;
use std::str::FromStr;
use std::sync::Arc;
use warg_api::package::PackageError;
use warg_api::{
    content::ContentSourceKind,
    package::{PendingRecordResponse, PublishRequest, RecordResponse},
};
use warg_crypto::hash::{DynHash, Sha256};
use warg_protocol::registry::{LogId, RecordId};

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

    pub fn into_router(self) -> Router {
        Router::new()
            .route("/", post(publish))
            .route("/:package_id/records/:record_id", get(get_record))
            .route("/:package_id/pending/:record_id", get(get_pending_record))
            .with_state(self)
    }
}

fn record_url(package_id: &LogId, record_id: &RecordId) -> String {
    format!("/package/{package_id}/records/{record_id}")
}

fn pending_record_url(package_id: &LogId, record_id: &RecordId) -> String {
    format!("/package/{package_id}/pending/{record_id}")
}

struct PackageApiError(PackageError);

impl From<CoreServiceError> for PackageApiError {
    fn from(e: CoreServiceError) -> Self {
        Self(match e {
            CoreServiceError::DataStore(e) => match e {
                crate::datastore::DataStoreError::PackageValidationFailed(e) => {
                    PackageError::InvalidRecord {
                        message: e.to_string(),
                    }
                }
                crate::datastore::DataStoreError::LogNotFound(log_id) => {
                    PackageError::PackageIdNotFound { log_id }
                }
                crate::datastore::DataStoreError::RecordNotFound(id) => {
                    PackageError::PackageRecordNotFound { id }
                }
                e => {
                    tracing::error!("unexpected data store error: {e}");
                    PackageError::Operation
                }
            },
            CoreServiceError::PackageNotFound(name) => PackageError::PackageNotFound { name },
        })
    }
}

impl IntoResponse for PackageApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self.0 {
            PackageError::InvalidPackageId { .. }
            | PackageError::InvalidRecordId { .. }
            | PackageError::InvalidRecord { .. }
            | PackageError::FailedToFetchContent { .. }
            | PackageError::ContentFetchErrorResponse { .. }
            | PackageError::ContentUrlInvalid { .. } => StatusCode::BAD_REQUEST,
            PackageError::PackageNotFound { .. }
            | PackageError::PackageIdNotFound { .. }
            | PackageError::PackageRecordNotFound { .. } => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self.0)).into_response()
    }
}

#[debug_handler]
async fn publish(
    State(config): State<Config>,
    Json(body): Json<PublishRequest>,
) -> Result<Json<PendingRecordResponse>, PackageApiError> {
    let record = body.record.try_into().map_err(|e: Error| {
        PackageApiError(PackageError::InvalidRecord {
            message: e.to_string(),
        })
    })?;

    for source in body.content_sources.iter() {
        match &source.kind {
            ContentSourceKind::HttpAnonymous { url } => {
                if url.starts_with(&config.base_url) {
                    let response = Client::builder()
                        .build()
                        .map_err(|e| {
                            PackageApiError(PackageError::FailedToFetchContent {
                                message: e.to_string(),
                            })
                        })?
                        .head(url)
                        .send()
                        .await
                        .map_err(|e| {
                            PackageApiError(PackageError::FailedToFetchContent {
                                message: e.to_string(),
                            })
                        })?;
                    if !response.status().is_success() {
                        return Err(PackageApiError(PackageError::ContentFetchErrorResponse {
                            status_code: response.status().as_u16(),
                        }));
                    }
                } else {
                    return Err(PackageApiError(PackageError::ContentUrlInvalid {
                        url: url.clone(),
                    }));
                }
            }
        }
    }

    let package_id = LogId::package_log::<Sha256>(&body.name);
    let record_id = RecordId::package_record::<Sha256>(&record);

    config
        .core_service
        .submit_package_record(&body.name, record, body.content_sources)
        .await?;

    Ok(Json(PendingRecordResponse::Processing {
        status_url: pending_record_url(&package_id, &record_id),
    }))
}

#[debug_handler]
async fn get_record(
    State(config): State<Config>,
    Path((package_id, record_id)): Path<(String, String)>,
) -> Result<Json<RecordResponse>, PackageApiError> {
    let package_id: LogId = DynHash::from_str(&package_id)
        .map_err(|e| {
            PackageApiError(PackageError::InvalidPackageId {
                message: e.to_string(),
            })
        })?
        .into();
    let record_id: RecordId = DynHash::from_str(&record_id)
        .map_err(|e| {
            PackageApiError(PackageError::InvalidRecordId {
                message: e.to_string(),
            })
        })?
        .into();

    let PackageLogEntry {
        record,
        sources,
        checkpoint,
    } = config
        .core_service
        .get_package_log_entry(&package_id, &record_id)
        .await?;

    Ok(Json(RecordResponse {
        record: record.into(),
        content_sources: sources,
        checkpoint,
    }))
}

#[debug_handler]
async fn get_pending_record(
    State(config): State<Config>,
    Path((package_id, record_id)): Path<(String, String)>,
) -> Result<Json<PendingRecordResponse>, PackageApiError> {
    let package_id: LogId = DynHash::from_str(&package_id)
        .map_err(|e| {
            PackageApiError(PackageError::InvalidPackageId {
                message: e.to_string(),
            })
        })?
        .into();
    let record_id: RecordId = DynHash::from_str(&record_id)
        .map_err(|e| {
            PackageApiError(PackageError::InvalidRecordId {
                message: e.to_string(),
            })
        })?
        .into();

    Ok(Json(
        match config
            .core_service
            .get_record_status(&package_id, &record_id)
            .await?
        {
            // Validated is considered still pending until the record is part of a checkpoint
            RecordStatus::Pending | RecordStatus::Validated => PendingRecordResponse::Processing {
                status_url: pending_record_url(&package_id, &record_id),
            },
            RecordStatus::Rejected(reason) => PendingRecordResponse::Rejected { reason },
            RecordStatus::InCheckpoint => PendingRecordResponse::Published {
                record_url: record_url(&package_id, &record_id),
            },
        },
    ))
}
