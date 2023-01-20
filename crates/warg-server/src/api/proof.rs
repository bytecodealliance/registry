use anyhow::Result;
use axum::{debug_handler, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use crate::{services::data, AnyError};

#[derive(Clone)]
pub struct Config {
    log: data::LogData,
    map: data::MapData,
}

impl Config {
    pub fn new(log: data::LogData, map: data::MapData) -> Self {
        Self { log, map }
    }

    pub fn build_router(self) -> Router {
        Router::new()
            .route("/log/consistency", post(log_consistency::prove))
            .route("/inclusion", post(inclusion::prove))
            .with_state(self)
    }
}

mod log_consistency {
    use axum::extract::State;

    use super::*;

    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestBody {
        old_root: String,
        new_root: String,
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
        let response = ResponseBody { proof: todo!() };

        Ok((StatusCode::OK, Json(response)))
    }
}

mod inclusion {
    use warg_crypto::hash::DynHash;

    use super::*;

    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestBody {
        root: String,
        logs: Vec<LogHead>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct LogHead {
        name: String,
        head: DynHash,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct ResponseBody {
        proof: Vec<u8>,
    }

    #[debug_handler]
    pub(crate) async fn prove(
        Json(body): Json<RequestBody>,
    ) -> Result<impl IntoResponse, AnyError> {
        let response = ResponseBody { proof: todo!() };

        Ok((StatusCode::OK, Json(response)))
    }
}
