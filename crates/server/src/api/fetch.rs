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
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use warg_crypto::hash::DynHash;
use warg_protocol::registry::{MapCheckpoint, RecordId};
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

#[derive(Debug, Serialize, Deserialize)]
pub struct FetchRequest {
    pub root: DynHash,
    pub operator: Option<RecordId>,
    pub packages: IndexMap<String, Option<RecordId>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FetchResponse {
    pub operator: Vec<ProtoEnvelopeBody>,
    pub packages: IndexMap<String, Vec<ProtoEnvelopeBody>>,
}

#[debug_handler]
async fn fetch_logs(
    State(config): State<Config>,
    Json(body): Json<FetchRequest>,
) -> Result<impl IntoResponse, AnyError> {
    let operator = config
        .core_service
        .fetch_operator_records(body.root.clone(), body.operator)
        .await?;
    let operator = operator
        .into_iter()
        .map(|env| env.as_ref().clone().into())
        .collect();

    let mut packages = IndexMap::new();
    for (name, since) in body.packages.into_iter() {
        let records = config
            .core_service
            .fetch_package_records(body.root.clone(), name.clone(), since)
            .await?
            .into_iter()
            .map(|env| env.as_ref().clone().into())
            .collect();
        packages.insert(name, records);
    }
    let response = FetchResponse { operator, packages };
    Ok((StatusCode::OK, Json(response)))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckpointResponse {
    pub checkpoint: SerdeEnvelope<MapCheckpoint>,
}

#[debug_handler]
async fn fetch_checkpoint(State(config): State<Config>) -> Result<impl IntoResponse, AnyError> {
    let checkpoint = config
        .core_service
        .get_latest_checkpoint()
        .await
        .as_ref()
        .to_owned();
    let response = CheckpointResponse { checkpoint };
    Ok((StatusCode::OK, Json(response)))
}
