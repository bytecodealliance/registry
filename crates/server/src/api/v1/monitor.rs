use super::{Json, RegistryHeader};
use crate::datastore::DataStoreError;
use crate::services::CoreService;
use axum::http::StatusCode;
use axum::{debug_handler, extract::State, response::IntoResponse, routing::post, Router};
use warg_api::v1::monitor::{CheckpointVerificationResponse, MonitorError};
use warg_crypto::hash::Sha256;
use warg_protocol::registry::{LogId, TimestampedCheckpoint};
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
            DataStoreError::UnknownKey(key_id) => {
                MonitorError::CheckpointSignatureKeyIdNotFound(key_id)
            }
            DataStoreError::SignatureVerificationFailed(signature) => {
                MonitorError::CheckpointSignatureInvalid(signature)
            }
            DataStoreError::KeyUnauthorized(key_id) => {
                MonitorError::CheckpointSignatureKeyIdUnauthorized(key_id)
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
    // look up checkpoint, if not found returns CheckpointNotFound
    let checkpoint = config
        .core_service
        .store()
        .get_checkpoint(body.as_ref().checkpoint.log_length)
        .await?;

    // if exact match, return Verified
    if checkpoint.signature() == body.signature()
        && checkpoint.key_id() == body.key_id()
        && checkpoint.as_ref().checkpoint.log_root == body.as_ref().checkpoint.log_root
        && checkpoint.as_ref().checkpoint.map_root == body.as_ref().checkpoint.map_root
    {
        return Ok(Json(CheckpointVerificationResponse::Verified));
    }

    // verify signature, which may return:
    //  - CheckpointSignatureKeyIdNotFound
    //  - CheckpointSignatureKeyIdUnauthorized
    //  - CheckpointSignatureInvalid
    config
        .core_service
        .store()
        .verify_timestamped_checkpoint_signature(&LogId::operator_log::<Sha256>(), &body)
        .await?;

    // verify log root, if not returns CheckpointLogRootInvalid
    if checkpoint.as_ref().checkpoint.log_root != body.as_ref().checkpoint.log_root {
        return Err(MonitorApiError(MonitorError::CheckpointLogRootInvalid(
            body.as_ref().checkpoint.log_root.clone(),
        )));
    }

    // verify map root, if not returns CheckpointMapRootInvalid
    if checkpoint.as_ref().checkpoint.map_root != body.as_ref().checkpoint.map_root {
        return Err(MonitorApiError(MonitorError::CheckpointMapRootInvalid(
            body.as_ref().checkpoint.map_root.clone(),
        )));
    }

    Ok(Json(CheckpointVerificationResponse::Verified))
}
