use crate::services::data::{self, DataServiceError};
use anyhow::{Error, Result};
use axum::{
    body::boxed, debug_handler, extract::State, http::StatusCode, response::IntoResponse,
    routing::post, Json, Router,
};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use warg_api::proof::{
    ConsistencyRequest, ConsistencyResponse, InclusionRequest, InclusionResponse,
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
            .with_state(self)
    }
}

#[derive(Debug, Error)]
pub(crate) enum ProofApiError {
    #[error("invalid old root: {0}")]
    InvalidOldRoot(Error),
    #[error("invalid new root: {0}")]
    InvalidNewRoot(Error),
    #[error("invalid log root: {0}")]
    InvalidLogRoot(Error),
    #[error("invalid map root: {0}")]
    InvalidMapRoot(Error),
    #[error("{0}")]
    DataService(#[from] DataServiceError),
}

impl IntoResponse for ProofApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            Self::InvalidOldRoot(_)
            | Self::InvalidNewRoot(_)
            | Self::InvalidLogRoot(_)
            | Self::InvalidMapRoot(_) => StatusCode::BAD_REQUEST,
            Self::DataService(_) => match self {
                Self::DataService(e) => return e.into_response(),
                _ => unreachable!(),
            },
        };

        axum::response::Response::builder()
            .status(status)
            .body(boxed(self.to_string()))
            .unwrap()
    }
}

#[debug_handler]
pub(crate) async fn prove_consistency(
    State(config): State<Config>,
    Json(body): Json<ConsistencyRequest>,
) -> Result<Json<ConsistencyResponse>, ProofApiError> {
    let log = config.log.as_ref().read().await;

    let old_root: Hash<Sha256> = body
        .old_root
        .try_into()
        .map_err(ProofApiError::InvalidOldRoot)?;
    let new_root: Hash<Sha256> = body
        .new_root
        .try_into()
        .map_err(ProofApiError::InvalidNewRoot)?;

    let bundle = log.consistency(&old_root, &new_root)?;

    Ok(Json(ConsistencyResponse {
        proof: bundle.encode(),
    }))
}

#[debug_handler]
pub(crate) async fn prove_inclusion(
    State(config): State<Config>,
    Json(body): Json<InclusionRequest>,
) -> Result<Json<InclusionResponse>, ProofApiError> {
    let log_root = body
        .checkpoint
        .log_root
        .try_into()
        .map_err(ProofApiError::InvalidLogRoot)?;
    let map_root = body
        .checkpoint
        .map_root
        .try_into()
        .map_err(ProofApiError::InvalidMapRoot)?;

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
