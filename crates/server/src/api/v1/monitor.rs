use super::{Json, RegistryHeader};
use crate::datastore::DataStoreError;
use crate::services::CoreService;
use axum::http::StatusCode;
use axum::{debug_handler, extract::State, response::IntoResponse, routing::post, Router};
use warg_api::v1::monitor::{CheckpointVerificationResponse, MonitorError};
use warg_protocol::registry::TimestampedCheckpoint;
use warg_protocol::SerdeEnvelope;

#[derive(Clone)]
pub struct Config {
    core_service: CoreService,
}

impl Config {
    pub fn new(core_service: CoreService) -> Self {
        Self { core_service }
    }

    pub fn into_router(self) -> Router {
        Router::new()
            .route("/checkpoint", post(verify_checkpoint))
            .with_state(self)
    }
}

struct MonitorApiError(MonitorError);

impl From<DataStoreError> for MonitorApiError {
    fn from(e: DataStoreError) -> Self {
        Self(match e {
            DataStoreError::CheckpointNotFound(log_length) => {
                MonitorError::CheckpointNotFound(log_length)
            }
            e => {
                tracing::error!("unexpected data store error: {e}");
                MonitorError::Message {
                    status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    message: "an error occurred while processing the request".into(),
                }
            }
        })
    }
}

impl IntoResponse for MonitorApiError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::from_u16(self.0.status()).unwrap(), Json(self.0)).into_response()
    }
}

#[debug_handler]
async fn verify_checkpoint(
    State(config): State<Config>,
    RegistryHeader(_registry_header): RegistryHeader,
    Json(body): Json<SerdeEnvelope<TimestampedCheckpoint>>,
) -> Result<Json<CheckpointVerificationResponse>, MonitorApiError> {
    let checkpoint = config
        .core_service
        .store()
        .get_checkpoint(body.as_ref().checkpoint.log_length)
        .await?;

    if checkpoint.key_id() != body.key_id() {
        return Err(MonitorApiError(
            MonitorError::CheckpointSignatureKeyIdInvalid(body.key_id().clone()),
        ));
    }

    if checkpoint.as_ref().checkpoint.log_root != body.as_ref().checkpoint.log_root {
        return Err(MonitorApiError(MonitorError::CheckpointLogRootIncorrect(
            body.as_ref().checkpoint.log_root.clone(),
        )));
    }

    if checkpoint.as_ref().checkpoint.map_root != body.as_ref().checkpoint.map_root {
        return Err(MonitorApiError(MonitorError::CheckpointMapRootIncorrect(
            body.as_ref().checkpoint.map_root.clone(),
        )));
    }

    if checkpoint.signature() != body.signature() {
        return Err(MonitorApiError(MonitorError::CheckpointSignatureInvalid(
            body.signature().clone(),
        )));
    }

    Ok(Json(CheckpointVerificationResponse::Verified))
}
