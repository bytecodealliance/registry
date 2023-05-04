use super::Json;
use crate::services::{DataServiceError, LogData, MapData};
use axum::{
    debug_handler, extract::State, http::StatusCode, response::IntoResponse, routing::post, Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use warg_api::v1::proof::{
    ConsistencyRequest, ConsistencyResponse, InclusionRequest, InclusionResponse, ProofError,
};
use warg_crypto::hash::{Hash, Sha256};

#[derive(Clone)]
pub struct Config {
    log: Arc<RwLock<LogData>>,
    map: Arc<RwLock<MapData>>,
}

impl Config {
    pub fn new(log: Arc<RwLock<LogData>>, map: Arc<RwLock<MapData>>) -> Self {
        Self { log, map }
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

impl From<DataServiceError> for ProofApiError {
    fn from(value: DataServiceError) -> Self {
        Self(match value {
            DataServiceError::RootNotFound(root) => ProofError::RootNotFound(root.into()),
            DataServiceError::LeafNotFound(leaf) => ProofError::LeafNotFound(leaf),
            DataServiceError::BundleFailure(e) => ProofError::BundleFailure(e.to_string()),
            DataServiceError::PackageNotIncluded(id) => ProofError::PackageLogNotIncluded(id),
            DataServiceError::IncorrectProof { root, found } => ProofError::IncorrectProof {
                root: root.into(),
                found: found.into(),
            },
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
    let log = config.log.as_ref().read().await;

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

    let bundle = log.consistency(&from, &to)?;

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

    let log_bundle = {
        let log = config.log.as_ref().read().await;
        log.inclusion(&log_root, body.leafs.as_ref())?
    };

    let map_bundle = {
        let map = config.map.as_ref().read().await;
        map.inclusion(&map_root, body.leafs.as_ref())?
    };

    Ok(Json(InclusionResponse {
        log: log_bundle.encode(),
        map: map_bundle.encode(),
    }))
}
