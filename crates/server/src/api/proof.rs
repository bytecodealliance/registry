use crate::{services::data, AnyError};
use anyhow::Result;
use axum::{
    debug_handler, extract::State, http::StatusCode, response::IntoResponse, routing::post, Json,
    Router,
};
use std::sync::Arc;
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

#[debug_handler]
pub(crate) async fn prove_consistency(
    State(config): State<Config>,
    Json(body): Json<ConsistencyRequest>,
) -> Result<impl IntoResponse, AnyError> {
    let log = config.log.as_ref().read().await;

    let old_root: Hash<Sha256> = body.old_root.try_into()?;
    let new_root: Hash<Sha256> = body.new_root.try_into()?;

    let bundle = log.consistency(old_root, new_root)?;

    let response = ConsistencyResponse {
        proof: bundle.encode(),
    };

    Ok((StatusCode::OK, Json(response)))
}

#[debug_handler]
pub(crate) async fn prove_inclusion(
    State(config): State<Config>,
    Json(body): Json<InclusionRequest>,
) -> Result<impl IntoResponse, AnyError> {
    let log_root: Hash<Sha256> = body.checkpoint.log_root.try_into()?;
    let map_root = body.checkpoint.map_root.try_into()?;

    let log_bundle = {
        let log = config.log.as_ref().read().await;
        log.inclusion(log_root, body.heads.as_slice())?
    };

    let map_bundle = {
        let map = config.map.as_ref().read().await;
        map.inclusion(map_root, body.heads.as_slice())?
    };

    let response = InclusionResponse {
        log: log_bundle.encode(),
        map: map_bundle.encode(),
    };

    Ok((StatusCode::OK, Json(response)))
}
