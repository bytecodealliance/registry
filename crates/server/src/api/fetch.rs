use crate::datastore::DataStoreError;
use crate::services::{CoreService, CoreServiceError};
use axum::http::StatusCode;
use axum::{
    debug_handler,
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use indexmap::IndexMap;
use std::sync::Arc;
use warg_api::fetch::{CheckpointResponse, FetchError, FetchRequest, FetchResponse};

#[derive(Clone)]
pub struct Config {
    core_service: Arc<CoreService>,
}

impl Config {
    pub fn new(core_service: Arc<CoreService>) -> Self {
        Self { core_service }
    }

    pub fn into_router(self) -> Router {
        Router::new()
            .route("/logs", post(fetch_logs))
            .route("/checkpoint", get(fetch_checkpoint))
            .with_state(self)
    }
}

struct FetchApiError(FetchError);

impl From<CoreServiceError> for FetchApiError {
    fn from(e: CoreServiceError) -> Self {
        Self(match e {
            CoreServiceError::DataStore(e) => match e {
                DataStoreError::CheckpointNotFound(checkpoint) => {
                    FetchError::CheckpointNotFound { checkpoint }
                }
                DataStoreError::LogNotFound(log_id) => FetchError::LogNotFound { log_id },
                DataStoreError::RecordNotFound(record_id) => {
                    FetchError::RecordNotFound { record_id }
                }

                // Other errors are unexpected operational errors
                e => {
                    tracing::error!("unexpected data store error: {e}");
                    FetchError::Operation
                }
            },
            CoreServiceError::PackageNotFound(name) => FetchError::PackageNotFound { name },
        })
    }
}

impl IntoResponse for FetchApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self.0 {
            FetchError::CheckpointNotFound { .. }
            | FetchError::LogNotFound { .. }
            | FetchError::PackageNotFound { .. }
            | FetchError::RecordNotFound { .. } => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self.0)).into_response()
    }
}

#[debug_handler]
async fn fetch_logs(
    State(config): State<Config>,
    Json(body): Json<FetchRequest>,
) -> Result<Json<FetchResponse>, FetchApiError> {
    let operator = config
        .core_service
        .fetch_operator_records(&body.root, body.since.as_ref())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    let mut packages = IndexMap::new();
    for (name, since) in body.packages.into_iter() {
        let records = config
            .core_service
            .fetch_package_records(&name, &body.root, since.as_ref())
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
        packages.insert(name, records);
    }

    Ok(Json(FetchResponse { operator, packages }))
}

#[debug_handler]
async fn fetch_checkpoint(
    State(config): State<Config>,
) -> Result<Json<CheckpointResponse>, FetchApiError> {
    let checkpoint = config.core_service.get_latest_checkpoint().await?;
    Ok(Json(CheckpointResponse { checkpoint }))
}
