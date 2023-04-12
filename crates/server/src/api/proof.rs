use crate::services::data::{self, DataServiceError};
use anyhow::Error;
use axum::{
    debug_handler, extract::State, http::StatusCode, response::IntoResponse, routing::post, Json,
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use warg_api::proof::{
    ConsistencyRequest, ConsistencyResponse, InclusionRequest, InclusionResponse, ProofError,
};
use warg_crypto::hash::{Hash, Sha256};

#[derive(Clone)]
pub struct Config {
    log: Arc<RwLock<data::log::ProofData>>,
    map: Arc<RwLock<data::map::MapData>>,
}

impl Config {
    pub fn new(
        log: Arc<RwLock<data::log::ProofData>>,
        map: Arc<RwLock<data::map::MapData>>,
    ) -> Self {
        Self { log, map }
    }

    pub fn build_router(self) -> Router {
        Router::new()
            .route("/consistency", post(prove_consistency))
            .route("/inclusion", post(prove_inclusion))
            .layer(CorsLayer::permissive())
            .with_state(self)
    }
}

struct ProofApiError(ProofError);

impl From<DataServiceError> for ProofApiError {
    fn from(value: DataServiceError) -> Self {
        Self(match value {
            DataServiceError::RootNotFound(root) => ProofError::RootNotFound { root },
            DataServiceError::LeafNotFound(leaf) => ProofError::LeafNotFound { leaf },
            DataServiceError::BundleFailure(e) => ProofError::BundleFailure {
                message: e.to_string(),
            },
            DataServiceError::PackageNotIncluded(id) => ProofError::PackageNotIncluded { id },
            DataServiceError::IncorrectProof { root, found } => {
                ProofError::IncorrectProof { root, found }
            }
        })
    }
}

impl IntoResponse for ProofApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self.0 {
            ProofError::InvalidLogRoot { .. }
            | ProofError::InvalidMapRoot { .. }
            | ProofError::BundleFailure { .. }
            | ProofError::PackageNotIncluded { .. }
            | ProofError::IncorrectProof { .. } => StatusCode::BAD_REQUEST,
            ProofError::RootNotFound { .. } | ProofError::LeafNotFound { .. } => {
                StatusCode::NOT_FOUND
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(self.0)).into_response()
    }
}

#[debug_handler]
async fn prove_consistency(
    State(config): State<Config>,
    Json(body): Json<ConsistencyRequest>,
) -> Result<Json<ConsistencyResponse>, ProofApiError> {
    println!("MADE IT TO HANDLER");
    let log = config.log.as_ref().read().await;

    let old_root: Hash<Sha256> = body.old_root.try_into().map_err(|e: Error| {
        ProofApiError(ProofError::InvalidLogRoot {
            message: e.to_string(),
        })
    })?;
    let new_root: Hash<Sha256> = body.new_root.try_into().map_err(|e: Error| {
        ProofApiError(ProofError::InvalidLogRoot {
            message: e.to_string(),
        })
    })?;

    let bundle = log.consistency(&old_root, &new_root)?;

    Ok(Json(ConsistencyResponse {
        proof: bundle.encode(),
    }))
}

#[debug_handler]
async fn prove_inclusion(
    State(config): State<Config>,
    Json(body): Json<InclusionRequest>,
) -> Result<Json<InclusionResponse>, ProofApiError> {
    let log_root = body.checkpoint.log_root.try_into().map_err(|e: Error| {
        ProofApiError(ProofError::InvalidLogRoot {
            message: e.to_string(),
        })
    })?;
    let map_root = body.checkpoint.map_root.try_into().map_err(|e: Error| {
        ProofApiError(ProofError::InvalidMapRoot {
            message: e.to_string(),
        })
    })?;

    let log_bundle = {
        let log = config.log.as_ref().read().await;
        log.inclusion(&log_root, body.heads.as_slice())?
    };

    let map_bundle = {
        let map = config.map.as_ref().read().await;
        map.inclusion(&map_root, body.heads.as_slice())?
    };

    Ok(Json(InclusionResponse {
        log: log_bundle.encode(),
        map: map_bundle.encode(),
    }))
}
