use super::{Json, RegistryHeader};
use crate::datastore::DataStoreError;
use crate::services::CoreService;
use axum::http::StatusCode;
use axum::{debug_handler, extract::State, response::IntoResponse, routing::post, Router};
use warg_api::v1::monitor::{
    CheckpointVerificationResponse, CheckpointVerificationState, MonitorError,
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
        (StatusCode::from_u16(self.0.status()).unwrap(), Json(self.0)).into_response()
    }
}

#[debug_handler]
async fn verify_checkpoint(
    State(config): State<Config>,
    RegistryHeader(_registry_header): RegistryHeader,
    Json(body): Json<SerdeEnvelope<TimestampedCheckpoint>>,
) -> Result<Json<CheckpointVerificationResponse>, MonitorApiError> {
    // check checkpoint:
    // - if `log_length` not found, `checkpoint` is `Invalid`;
    // - if `log_root` or `map_root` is incorrect, `checkpoint` is `Invalid`;
    // - if `key_id` and `signature` does not match previously stored value,
    //   `signature` is `Unverified` and will check against the operator log;
    let (checkpoint_verification, mut signature_verification) = match config
        .core_service
        .store()
        .get_checkpoint(body.as_ref().checkpoint.log_length)
        .await
    {
        Ok(checkpoint) => {
            // check log root and map root
            let checkpoint_verification = if checkpoint.as_ref().checkpoint.log_root
                == body.as_ref().checkpoint.log_root
                && checkpoint.as_ref().checkpoint.map_root == body.as_ref().checkpoint.map_root
            {
                CheckpointVerificationState::Verified
            } else {
                CheckpointVerificationState::Invalid
            };

            // check for exact match on signature and key ID
            let signature_verification = if checkpoint.signature() == body.signature()
                && checkpoint.key_id() == body.key_id()
            {
                CheckpointVerificationState::Verified
            } else {
                // set to Unverified and check against operator log keys below
                CheckpointVerificationState::Unverified
            };

            (checkpoint_verification, signature_verification)
        }
        Err(DataStoreError::CheckpointNotFound(_)) => (
            CheckpointVerificationState::Invalid,
            CheckpointVerificationState::Unverified,
        ),
        Err(error) => return Err(MonitorApiError::from(error)),
    };

    // if `Unverified`, check signature against keys in operator log:
    //
    // - if `key_id` is not known or it does not have permission to sign checkpoints or
    //   the `signature` is invalid, `signature` is `Invalid`;
    if signature_verification == CheckpointVerificationState::Unverified {
        match config
            .core_service
            .store()
            .verify_timestamped_checkpoint_signature(&LogId::operator_log::<Sha256>(), &body)
            .await
        {
            Ok(_) => {
                signature_verification = CheckpointVerificationState::Verified;
            }
            Err(DataStoreError::UnknownKey(_))
            | Err(DataStoreError::SignatureVerificationFailed(_))
            | Err(DataStoreError::KeyUnauthorized(_)) => {
                signature_verification = CheckpointVerificationState::Invalid;
            }
            Err(error) => return Err(MonitorApiError::from(error)),
        };
    }

    Ok(Json(CheckpointVerificationResponse {
        checkpoint: checkpoint_verification,
        signature: signature_verification,
        retry_after: None,
    }))
}
