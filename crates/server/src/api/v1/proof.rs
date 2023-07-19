use super::Json;
use crate::services::{CoreService, CoreServiceError};
use axum::{
    debug_handler, extract::State, http::StatusCode, response::IntoResponse, routing::post, Router,
};
use warg_api::v1::proof::{
    ConsistencyRequest, ConsistencyResponse, InclusionRequest, InclusionResponse, ProofError,
};
use warg_crypto::hash::{Hash, Sha256};

#[derive(Clone)]
pub struct Config {
    core: CoreService,
}

impl Config {
    pub fn new(core: CoreService) -> Self {
        Self { core }
    }

    pub fn into_router(self) -> Router {
        Router::new()
            .route("/consistency", post(prove_consistency))
            .route("/inclusion", post(prove_inclusion))
            .with_state(self)
    }
}

struct ProofApiError(ProofError);

impl ProofApiError {
    fn bad_request(message: impl ToString) -> Self {
        Self(ProofError::Message {
            status: StatusCode::BAD_REQUEST.as_u16(),
            message: message.to_string(),
        })
    }
}

impl From<CoreServiceError> for ProofApiError {
    fn from(value: CoreServiceError) -> Self {
        Self(match value {
            CoreServiceError::RootNotFound(root) => ProofError::RootNotFound(root),
            CoreServiceError::LeafNotFound(leaf) => ProofError::LeafNotFound(leaf),
            CoreServiceError::BundleFailure(e) => ProofError::BundleFailure(e.to_string()),
            CoreServiceError::PackageNotIncluded(id) => ProofError::PackageLogNotIncluded(id),
            CoreServiceError::IncorrectProof { root, found } => {
                ProofError::IncorrectProof { root, found }
            }
            other => {
                tracing::error!("Unhandled CoreServiceError: {other:?}");
                ProofError::Message {
                    status: 500,
                    message: "Internal service error".to_string(),
                }
            }
        })
    }
}

impl IntoResponse for ProofApiError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::from_u16(self.0.status()).unwrap(), Json(self.0)).into_response()
    }
}

#[debug_handler]
async fn prove_consistency(
    State(config): State<Config>,
    Json(body): Json<ConsistencyRequest<'static>>,
) -> Result<Json<ConsistencyResponse>, ProofApiError> {
    let from: Hash<Sha256> = body
        .from
        .into_owned()
        .try_into()
        .map_err(ProofApiError::bad_request)?;
    let to: Hash<Sha256> = body
        .to
        .into_owned()
        .try_into()
        .map_err(ProofApiError::bad_request)?;

    let bundle = config.core.log_consistency_proof(&from, &to).await?;

    Ok(Json(ConsistencyResponse {
        proof: bundle.encode(),
    }))
}

#[debug_handler]
async fn prove_inclusion(
    State(config): State<Config>,
    Json(body): Json<InclusionRequest<'static>>,
) -> Result<Json<InclusionResponse>, ProofApiError> {
    let checkpoint = body.checkpoint.into_owned();
    let log_root = checkpoint
        .log_root
        .try_into()
        .map_err(ProofApiError::bad_request)?;
    let map_root = checkpoint
        .map_root
        .try_into()
        .map_err(ProofApiError::bad_request)?;

    let log_bundle = config
        .core
        .log_inclusion_proofs(&log_root, &body.leafs)
        .await?;

    let map_bundle = config
        .core
        .map_inclusion_proofs(&map_root, &body.leafs)
        .await?;

    Ok(Json(InclusionResponse {
        log: log_bundle.encode(),
        map: map_bundle.encode(),
    }))
}
