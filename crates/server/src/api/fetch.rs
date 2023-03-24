use crate::services::core::{CoreService, CoreServiceError};
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

    pub fn build_router(self) -> Router {
        Router::new()
            .route("/logs", post(fetch_logs))
            .route("/checkpoint", get(fetch_checkpoint))
            .with_state(self)
    }
}

struct FetchApiError(FetchError);

impl From<CoreServiceError> for FetchApiError {
    fn from(value: CoreServiceError) -> Self {
        Self(match value {
            CoreServiceError::CheckpointNotFound(checkpoint) => {
                FetchError::CheckpointNotFound { checkpoint }
            }
            CoreServiceError::PackageNotFound(id) => FetchError::PackageNotFound { id },
            CoreServiceError::PackageRecordNotFound(id) => FetchError::PackageRecordNotFound { id },
            CoreServiceError::OperatorRecordNotFound(id) => {
                FetchError::OperatorRecordNotFound { id }
            }
            CoreServiceError::InvalidCheckpoint(e) => FetchError::InvalidCheckpoint {
                message: e.to_string(),
            },
        })
    }
}

impl IntoResponse for FetchApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self.0 {
            FetchError::CheckpointNotFound { .. }
            | FetchError::PackageNotFound { .. }
            | FetchError::PackageRecordNotFound { .. }
            | FetchError::OperatorRecordNotFound { .. } => StatusCode::NOT_FOUND,
            FetchError::InvalidCheckpoint { .. } => StatusCode::BAD_REQUEST,
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
        .fetch_operator_records(body.root.clone(), body.operator)
        .await?;
    let operator = operator
        .into_iter()
        .map(|env| env.as_ref().clone().into())
        .collect();

    let mut packages = IndexMap::new();
    for (name, since) in body.packages.into_iter() {
        let records = config
            .core_service
            .fetch_package_records(body.root.clone(), name.clone(), since)
            .await?
            .into_iter()
            .map(|env| env.as_ref().clone().into())
            .collect();
        packages.insert(name, records);
    }
    Ok(Json(FetchResponse { operator, packages }))
}

#[debug_handler]
async fn fetch_checkpoint(
    State(config): State<Config>,
) -> Result<Json<CheckpointResponse>, FetchApiError> {
    let checkpoint = config
        .core_service
        .get_latest_checkpoint()
        .await
        .as_ref()
        .to_owned();

    Ok(Json(CheckpointResponse { checkpoint }))
}
