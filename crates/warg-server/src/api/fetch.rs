use std::sync::Arc;

use anyhow::Result;
use axum::extract::State;
use axum::{
    debug_handler,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use warg_crypto::hash::DynHash;
use warg_protocol::registry::MapCheckpoint;
use warg_protocol::{ProtoEnvelopeBody, SerdeEnvelope};

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
        Router::new()
            .route("/logs", post(fetch_logs))
            .route("/checkpoint", get(fetch_checkpoint))
            .with_state(self)
    }
}

#[derive(Debug, Deserialize)]
pub struct RequestBody {
    root: DynHash,
    operator: Option<DynHash>,
    packages: Vec<LogSince>,
}

#[derive(Debug, Deserialize)]
struct LogSince {
    name: String,
    since: Option<DynHash>,
}

#[derive(Debug, Serialize)]
pub struct ResponseBody {
    operator: Vec<ProtoEnvelopeBody>,
    packages: Vec<PackageResults>,
}

#[derive(Debug, Serialize)]
struct PackageResults {
    name: String,
    records: Vec<ProtoEnvelopeBody>,
}

#[debug_handler]
async fn fetch_logs(
    State(config): State<Config>,
    Json(body): Json<RequestBody>,
) -> Result<impl IntoResponse, AnyError> {
    let mut packages = Vec::new();
    let operator = config
        .core_service
        .fetch_operator_records(body.root.clone(), body.operator)
        .await?;
    let operator = operator
        .into_iter()
        .map(|env| env.as_ref().clone().into())
        .collect();

    for LogSince { name, since } in body.packages {
        let records = config
            .core_service
            .fetch_package_records(body.root.clone(), name.clone(), since)
            .await?;
        let log_result = PackageResults {
            name: name.clone(),
            records: records
                .into_iter()
                .map(|env| env.as_ref().clone().into())
                .collect(),
        };
        packages.push(log_result);
    }
    let response = ResponseBody { operator, packages };
    Ok((StatusCode::OK, Json(response)))
}

#[derive(Debug, Serialize)]
struct CheckpointResponse {
    checkpoint: Arc<SerdeEnvelope<MapCheckpoint>>,
}

#[debug_handler]
async fn fetch_checkpoint(State(config): State<Config>) -> Result<impl IntoResponse, AnyError> {
    let checkpoint = config.core_service.get_latest_checkpoint().await;
    let response = CheckpointResponse { checkpoint };
    Ok((StatusCode::OK, Json(response)))
}
