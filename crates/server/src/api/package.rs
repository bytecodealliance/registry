use crate::services::core::{CoreService, CoreServiceError, PackageRecordInfo, RecordState};
use anyhow::{Error, Result};
use axum::body::boxed;
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
use thiserror::Error;
use warg_api::{
    content::ContentSourceKind,
    package::{PendingRecordResponse, PublishRequest, RecordResponse},
};
use warg_crypto::hash::{DynHash, DynHashParseError, Sha256};
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

    pub fn build_router(self) -> Router {
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

#[derive(Debug, Error)]
pub(crate) enum PackageApiError {
    #[error("invalid package id: {0}")]
    InvalidPackageId(DynHashParseError),
    #[error("invalid record id: {0}")]
    InvalidRecordId(DynHashParseError),
    #[error("invalid record: {0}")]
    InvalidRecord(Error),
    #[error("package record `{0}` not found")]
    RecordNotFound(RecordId),
    #[error("failed to fetch content: {0}")]
    FailedToFetchContent(reqwest::Error),
    #[error("cannot validate content source: {0} status returned from server")]
    ContentFetchErrorResponse(StatusCode),
    #[error("content source `{0}` is not from the current host")]
    ContentUrlInvalid(String),
    #[error("{0}")]
    CoreService(#[from] CoreServiceError),
}

impl IntoResponse for PackageApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            Self::InvalidPackageId(_)
            | Self::InvalidRecordId(_)
            | Self::InvalidRecord(_)
            | Self::FailedToFetchContent(_)
            | Self::ContentFetchErrorResponse(_)
            | Self::ContentUrlInvalid(_) => StatusCode::BAD_REQUEST,
            Self::RecordNotFound(_) => StatusCode::NOT_FOUND,
            Self::CoreService(_) => match self {
                Self::CoreService(e) => return e.into_response(),
                _ => unreachable!(),
            },
        };

        axum::response::Response::builder()
            .status(status)
            .body(boxed(self.to_string()))
            .unwrap()
    }
}

fn create_pending_response(
    package_id: &LogId,
    record_id: &RecordId,
    state: RecordState,
) -> Result<PendingRecordResponse, PackageApiError> {
    let response = match state {
        RecordState::Processing => PendingRecordResponse::Processing {
            status_url: pending_record_url(package_id, record_id),
        },
        RecordState::Published { .. } => PendingRecordResponse::Published {
            record_url: record_url(package_id, record_id),
        },
        RecordState::Rejected { reason } => PendingRecordResponse::Rejected { reason },
    };
    Ok(response)
}

#[debug_handler]
pub(crate) async fn publish(
    State(config): State<Config>,
    Json(body): Json<PublishRequest>,
) -> Result<Json<PendingRecordResponse>, PackageApiError> {
    let record = Arc::new(
        body.record
            .try_into()
            .map_err(PackageApiError::InvalidRecord)?,
    );
    let record_id = RecordId::package_record::<Sha256>(&record);

    for source in body.content_sources.iter() {
        match &source.kind {
            ContentSourceKind::HttpAnonymous { url } => {
                if url.starts_with(&config.base_url) {
                    let response = Client::builder()
                        .build()
                        .map_err(PackageApiError::FailedToFetchContent)?
                        .head(url)
                        .send()
                        .await
                        .map_err(PackageApiError::FailedToFetchContent)?;
                    if !response.status().is_success() {
                        return Err(PackageApiError::ContentFetchErrorResponse(
                            response.status(),
                        ));
                    }
                } else {
                    return Err(PackageApiError::ContentUrlInvalid(url.clone()));
                }
            }
        }
    }

    let package_id = LogId::package_log::<Sha256>(&body.name);

    let state = config
        .core_service
        .submit_package_record(body.name, record, body.content_sources)
        .await;

    Ok(Json(create_pending_response(
        &package_id,
        &record_id,
        state,
    )?))
}

#[debug_handler]
pub(crate) async fn get_record(
    State(config): State<Config>,
    Path((log_id, record_id)): Path<(String, String)>,
) -> Result<Json<RecordResponse>, PackageApiError> {
    let log_id: LogId = DynHash::from_str(&log_id)
        .map_err(PackageApiError::InvalidPackageId)?
        .into();
    let record_id: RecordId = DynHash::from_str(&record_id)
        .map_err(PackageApiError::InvalidRecordId)?
        .into();

    match config
        .core_service
        .get_package_record_info(log_id, record_id.clone())
        .await?
    {
        PackageRecordInfo {
            record,
            content_sources,
            state: RecordState::Published { checkpoint },
        } => Ok(Json(RecordResponse {
            record: record.as_ref().clone().into(),
            content_sources,
            checkpoint,
        })),
        _ => Err(PackageApiError::RecordNotFound(record_id)),
    }
}

#[debug_handler]
pub(crate) async fn get_pending_record(
    State(config): State<Config>,
    Path((package_id, record_id)): Path<(String, String)>,
) -> Result<Json<PendingRecordResponse>, PackageApiError> {
    let package_id: LogId = DynHash::from_str(&package_id)
        .map_err(PackageApiError::InvalidPackageId)?
        .into();
    let record_id: RecordId = DynHash::from_str(&record_id)
        .map_err(PackageApiError::InvalidRecordId)?
        .into();

    let status = config
        .core_service
        .get_package_record_status(package_id.clone(), record_id.clone())
        .await?;

    Ok(Json(create_pending_response(
        &package_id,
        &record_id,
        status,
    )?))
}
