use super::{Json, RegistryHeader};
use crate::datastore::DataStoreError;
use crate::services::CoreService;
use axum::http::{header, StatusCode};
use axum::{debug_handler, extract::State, response::IntoResponse, routing::post, Router};
use warg_api::v1::monitor::{
    CheckpointSignatureVerificationState, CheckpointVerificationResponse,
    CheckpointVerificationState, MonitorError,
};
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
        tracing::error!("unexpected data store error: {e}");
        Self(MonitorError::Message {
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            message: "an error occurred while processing the request".into(),
        })
    }
}

impl IntoResponse for MonitorApiError {
    fn into_response(self) -> axum::response::Response {
        match self.0 {
            MonitorError::RetryAfter(seconds) => {
                let mut headers = header::HeaderMap::new();
                headers.insert(header::RETRY_AFTER, seconds.into());
                (
                    StatusCode::from_u16(self.0.status()).unwrap(),
                    headers,
                    Json(self.0),
                )
                    .into_response()
            }
            _ => (StatusCode::from_u16(self.0.status()).unwrap(), Json(self.0)).into_response(),
        }
    }
}

#[debug_handler]
async fn verify_checkpoint(
    State(config): State<Config>,
    RegistryHeader(_registry_header): RegistryHeader,
    Json(body): Json<SerdeEnvelope<TimestampedCheckpoint>>,
) -> Result<Json<CheckpointVerificationResponse>, MonitorApiError> {
    // look up checkpoint
    let (checkpoint_verification, mut signature_verification) = match config
        .core_service
        .store()
        .get_checkpoint(body.as_ref().checkpoint.log_length)
        .await
    {
        Ok(checkpoint) => {
            // if exact match, return Verified for both `checkpoint` and `signature`
            let checkpoint_verification = if checkpoint.as_ref().checkpoint.log_root
                == body.as_ref().checkpoint.log_root
                && checkpoint.as_ref().checkpoint.map_root == body.as_ref().checkpoint.map_root
            {
                CheckpointVerificationState::Verified
            } else {
                CheckpointVerificationState::Invalid
            };

            let signature_verification = if checkpoint.signature() == body.signature()
                && checkpoint.key_id() == body.key_id()
            {
                CheckpointSignatureVerificationState::Verified
            } else {
                // set to Unverified and check against operator log keys below
                CheckpointSignatureVerificationState::Unverified
            };

            (checkpoint_verification, signature_verification)
        }
        Err(DataStoreError::CheckpointNotFound(_)) => (
            CheckpointVerificationState::NotFound,
            CheckpointSignatureVerificationState::Unverified,
        ),
        Err(error) => return Err(MonitorApiError::from(error)),
    };

    // if Unverified, check signature against keys in operator log
    if signature_verification == CheckpointSignatureVerificationState::Unverified {
        match config
            .core_service
            .store()
            .verify_timestamped_checkpoint_signature(&LogId::operator_log::<Sha256>(), &body)
            .await
        {
            Ok(_) => {
                signature_verification = CheckpointSignatureVerificationState::Verified;
            }
            Err(DataStoreError::UnknownKey(_))
            | Err(DataStoreError::SignatureVerificationFailed(_)) => {
                signature_verification = CheckpointSignatureVerificationState::Invalid;
            }
            Err(DataStoreError::KeyUnauthorized(_)) => {
                signature_verification = CheckpointSignatureVerificationState::Unauthorized;
            }
            Err(error) => return Err(MonitorApiError::from(error)),
        };
    }

    Ok(Json(CheckpointVerificationResponse {
        checkpoint: checkpoint_verification,
        signature: signature_verification,
    }))
}
