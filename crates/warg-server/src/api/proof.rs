use std::sync::Arc;

use anyhow::Result;
use axum::extract::State;
use axum::{debug_handler, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use warg_crypto::hash::{DynHash, Hash, Sha256};
use warg_protocol::registry::{LogId, LogLeaf, MapCheckpoint, RecordId};

use crate::{services::data, AnyError};

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

#[derive(Serialize, Deserialize)]
pub struct ConsistencyRequest {
    old_root: DynHash,
    new_root: DynHash,
}

#[derive(Serialize, Deserialize)]
pub struct ConsistencyResponse {
    proof: Vec<u8>,
}

#[debug_handler]
pub(crate) async fn prove_consistency(
    State(config): State<Config>,
    Json(body): Json<ConsistencyRequest>,
) -> Result<impl IntoResponse, AnyError> {
    let log = config.log.as_ref().blocking_read();

    let old_root: Hash<Sha256> = body.old_root.try_into()?;
    let new_root: Hash<Sha256> = body.new_root.try_into()?;

    let bundle = log.consistency(old_root, new_root)?;

    let response = ConsistencyResponse {
        proof: bundle.encode(),
    };

    Ok((StatusCode::OK, Json(response)))
}

#[derive(Serialize, Deserialize)]
pub struct InclusionRequest {
    pub checkpoint: MapCheckpoint,
    pub heads: Vec<LogLeaf>,
}

#[derive(Serialize, Deserialize)]
pub struct InclusionResponse {
    pub log: Vec<u8>,
    pub map: Vec<u8>,
}

#[debug_handler]
pub(crate) async fn prove_inclusion(
    State(config): State<Config>,
    Json(body): Json<InclusionRequest>,
) -> Result<impl IntoResponse, AnyError> {
    let log_root: Hash<Sha256> = body.checkpoint.log_root.try_into()?;
    let map_root = body.checkpoint.map_root.try_into()?;

    let log_bundle = {
        let log = config.log.as_ref().blocking_read();
        log.inclusion(log_root, body.heads.as_slice())?
    };

    let map_bundle = {
        let map = config.map.as_ref().blocking_read();
        map.inclusion(map_root, body.heads.as_slice())?
    };

    let response = InclusionResponse {
        log: log_bundle.encode(),
        map: map_bundle.encode(),
    };

    Ok((StatusCode::OK, Json(response)))
}
