use std::sync::Arc;

use anyhow::Result;
use axum::extract::State;
use axum::{debug_handler, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use warg_crypto::hash::DynHash;
use warg_protocol::Envelope;

use crate::services::core::CoreService;
use crate::AnyError;

#[derive(Clone)]
pub struct Config {
    core_service: Arc<CoreService>,
}

impl Config {
    pub fn new(core_service: Arc<CoreService>) -> Self {
        Self { core_service }
    }

    pub fn build_router(self) -> Router {
        Router::new().route("/fetch", post(fetch)).with_state(self)
    }
}

use warg_protocol::package;

#[derive(Default, Debug, Deserialize)]
pub struct RequestBody {
    logs: Vec<LogSince>,
}

#[derive(Default, Debug, Deserialize)]
struct LogSince {
    name: String,
    since: Option<DynHash>,
}

#[derive(Default, Debug, Serialize)]
pub struct ResponseBody {
    logs: Vec<LogResults>,
}

#[derive(Default, Debug, Serialize)]
struct LogResults {
    name: String,
    records: Vec<Arc<Envelope<package::PackageRecord>>>,
}

#[debug_handler]
async fn fetch(
    State(config): State<Config>,
    Json(body): Json<RequestBody>,
) -> Result<impl IntoResponse, AnyError> {
    let mut logs = Vec::new();
    for LogSince { name, since } in body.logs {
        let records = config.core_service.fetch_since(name.clone(), since).await?;
        let log_result = LogResults {
            name: name.clone(),
            records,
        };
        logs.push(log_result);
    }
    let response = ResponseBody { logs };
    Ok((StatusCode::OK, Json(response)))
}
