use super::{Json, RegistryHeader};
use crate::datastore::DataStoreError;
use crate::services::CoreService;
use axum::http::StatusCode;
use axum::{debug_handler, extract::State, response::IntoResponse, routing::post, Router};
use warg_api::v1::monitor::{CheckpointVerificationResponse, MonitorError, VerificationState};
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

/// Verifies a checkpoint and its signature.
///
/// Note: Other implementations may choose to perform validation differently
/// and to respond with `Unverified` in cases where the required information
/// is not available or would be too expensive to compute.
///
/// Checkpoint verification is `Verified` when
/// a checkpoint with the provided `log_length` is found in the store
/// and both the log and map roots match the stored checkpoint.
///
/// Checkpoint verification is `Invalid` otherwise.
///
/// Signature verification is `Verified` when either
/// A) It matches the signature on the stored checkpoint, or
/// B) It is a valid and authorized signature by a key in the operator log.
///
/// Signature verification is `Invalid` otherwise.
#[debug_handler]
async fn verify_checkpoint(
    State(config): State<Config>,
    RegistryHeader(_registry_header): RegistryHeader,
    Json(body): Json<SerdeEnvelope<TimestampedCheckpoint>>,
) -> Result<Json<CheckpointVerificationResponse>, MonitorApiError> {
    // Do a first pass checking the provided checkpoint against the data store
    let (checkpoint_verification, signature_verification) =
        try_verify_exact_match(&config.core_service, &body).await;

    // If the signature is `Unverified`, check signature against keys in operator log:
    let signature_verification = if signature_verification == VerificationState::Unverified {
        match config
            .core_service
            .store()
            .verify_timestamped_checkpoint_signature(&LogId::operator_log::<Sha256>(), &body)
            .await
        {
            Ok(_) => VerificationState::Verified,
            Err(error) => match error {
                DataStoreError::UnknownKey(_)
                | DataStoreError::SignatureVerificationFailed(_)
                | DataStoreError::KeyUnauthorized(_) => VerificationState::Invalid,
                _ => return Err(MonitorApiError::from(error)),
            },
        }
    } else {
        signature_verification
    };

    Ok(Json(CheckpointVerificationResponse {
        checkpoint: checkpoint_verification,
        signature: signature_verification,
        retry_after: None,
    }))
}

/// Attempt to verify checkpoint by looking for an exact match in the store.
/// Returns (checkpoint: Invalid, signature: Unverified) if one isn't found.
async fn try_verify_exact_match(
    core_service: &CoreService,
    checkpoint_envelope: &SerdeEnvelope<TimestampedCheckpoint>,
) -> (VerificationState, VerificationState) {
    let checkpoint = &checkpoint_envelope.as_ref().checkpoint;

    // Look for a stored checkpoint with the same log_length as was specified
    let found = core_service
        .store()
        .get_checkpoint(checkpoint.log_length)
        .await;

    if let Ok(found_checkpoint_envelope) = found {
        let found_checkpoint = &found_checkpoint_envelope.as_ref().checkpoint;
        // Check log root and map root
        let log_matches = found_checkpoint.log_root == checkpoint.log_root;
        let map_matches = found_checkpoint.map_root == checkpoint.map_root;

        // A checkpoint is verified if the exact checkpoint was recorded in the store.
        // Otherwise it is considered invalid by the reference implementation.
        let checkpoint_verification = if log_matches && map_matches {
            VerificationState::Verified
        } else {
            VerificationState::Invalid
        };

        // Check for exact match on signature and key ID
        let signature_matches =
            found_checkpoint_envelope.signature() == checkpoint_envelope.signature();
        let key_id_matches = found_checkpoint_envelope.key_id() == checkpoint_envelope.key_id();

        // A checkpoint is verified if the signature and key_id match the found checkpoint.
        // Otherwise it is consdered unverified by this function, but can be checked against known keys afterwards.
        let signature_verification = if signature_matches && key_id_matches {
            VerificationState::Verified
        } else {
            VerificationState::Unverified
        };

        (checkpoint_verification, signature_verification)
    } else {
        (VerificationState::Invalid, VerificationState::Unverified)
    }
}
