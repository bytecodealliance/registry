use super::{Json, RegistryHeader};
use crate::services::{CoreService, CoreServiceError};
use axum::{
    debug_handler, extract::State, http::StatusCode, response::IntoResponse, routing::post, Router,
};
use warg_api::v1::proof::{
    ConsistencyRequest, ConsistencyResponse, InclusionRequest, InclusionResponse, ProofError,
};
use warg_protocol::registry::{RegistryIndex, RegistryLen};

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

impl From<CoreServiceError> for ProofApiError {
    fn from(value: CoreServiceError) -> Self {
        Self(match value {
            CoreServiceError::CheckpointNotFound(log_length) => {
                ProofError::CheckpointNotFound(log_length)
            }
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
    RegistryHeader(_registry_header): RegistryHeader,
    Json(body): Json<ConsistencyRequest>,
) -> Result<Json<ConsistencyResponse>, ProofApiError> {
    let bundle = config
        .core
        .log_consistency_proof(body.from as RegistryLen, body.to as RegistryLen)
        .await?;

    Ok(Json(ConsistencyResponse {
        proof: bundle.encode(),
    }))
}

#[debug_handler]
async fn prove_inclusion(
    State(config): State<Config>,
    RegistryHeader(_registry_header): RegistryHeader,
    Json(body): Json<InclusionRequest>,
) -> Result<Json<InclusionResponse>, ProofApiError> {
    let log_length = body.log_length as RegistryLen;
    let leafs = body
        .leafs
        .into_iter()
        .map(|index| index as RegistryIndex)
        .collect::<Vec<RegistryIndex>>();

    let log_bundle = config.core.log_inclusion_proofs(log_length, &leafs).await?;
    let map_bundle = config.core.map_inclusion_proofs(log_length, &leafs).await?;

    Ok(Json(InclusionResponse {
        log: log_bundle.encode(),
        map: map_bundle.encode(),
    }))
}
