use std::sync::Arc;

use anyhow::Result;
use axum::{debug_handler, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use axum::extract::State;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use warg_crypto::hash::{DynHash, Hash, Sha256};

use crate::{services::data, AnyError};

#[derive(Clone)]
pub struct Config {
    log: Arc<RwLock<data::log::ProofData>>,
    map: Arc<RwLock<data::map::MapData>>,
}

impl Config {
    pub fn new(
        log: Arc<RwLock<data::log::ProofData>>,
        map: Arc<RwLock<data::map::MapData>>,) -> Self {
        Self { log, map }
    }

    pub fn build_router(self) -> Router {
        Router::new()
            .route("/consistency/log", post(log_consistency::prove))
            .route("/inclusion", post(inclusion::prove))
            .with_state(self)
    }
}

mod log_consistency {
    use super::*;

    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestBody {
        old_root: DynHash,
        new_root: DynHash,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct ResponseBody {
        proof: Vec<u8>,
    }

    #[debug_handler]
    pub(crate) async fn prove(
        State(config): State<Config>,
        Json(body): Json<RequestBody>,
    ) -> Result<impl IntoResponse, AnyError> {
        let log = config.log.as_ref().blocking_read();

        let old_root: Hash<Sha256> = body.old_root.try_into()?;
        let new_root: Hash<Sha256> = body.new_root.try_into()?;

        let bundle = log.consistency(old_root, new_root)?;

        let response = ResponseBody { proof: bundle.encode() };

        Ok((StatusCode::OK, Json(response)))
    }
}

mod inclusion {
    use warg_crypto::hash::DynHash;
    use warg_protocol::registry::{LogLeaf, LogId, RecordId, MapCheckpoint};

    use super::*;

    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestBody {
        checkpoint: MapCheckpoint,
        logs: Vec<LogHead>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct LogHead {
        name: String,
        head: DynHash,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct ResponseBody {
        log: Vec<u8>,
        map: Vec<u8>
    }

    #[debug_handler]
    pub(crate) async fn prove(
        State(config): State<Config>,
        Json(body): Json<RequestBody>,
    ) -> Result<impl IntoResponse, AnyError> {
        let log = config.log.as_ref().blocking_read();
        let log_root: Hash<Sha256> = body.checkpoint.log_root.try_into()?;

        let mut leaves = Vec::new();
        for log_head in body.logs.iter() {
            leaves.push(LogLeaf {
                log_id: LogId::package_log::<Sha256>(&log_head.name),
                record_id: RecordId::from(log_head.head.clone()),
            });
        }

        let log_bundle = log.inclusion(log_root, leaves)?;
        drop(log);

        let map = config.map.as_ref().blocking_read();
        let map_root = body.checkpoint.map_root.try_into()?;

        let mut log_ids = Vec::new();
        for log_head in body.logs {
            log_ids.push(LogId::package_log::<Sha256>(&log_head.name))
        }

        let map_bundle = map.inclusion(map_root, log_ids)?;

        let response = ResponseBody { log: log_bundle.encode(), map: map_bundle.encode() };

        Ok((StatusCode::OK, Json(response)))
    }
}
