use std::sync::Arc;

use anyhow::Result;
use axum::extract::State;
use axum::{debug_handler, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use warg_crypto::hash::DynHash;
use warg_protocol::ProtoEnvelopeBody;

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

#[derive(Debug, Deserialize)]
pub struct RequestBody {
    root: DynHash,
    operator: Option<DynHash>,
    packages: Vec<LogSince>,
}

#[derive(Default, Debug, Deserialize)]
struct LogSince {
    name: String,
    since: Option<DynHash>,
}

#[derive(Default, Debug, Serialize)]
pub struct ResponseBody {
    operator: Vec<ProtoEnvelopeBody>,
    packages: Vec<PackageResults>,
}

#[derive(Default, Debug, Serialize)]
struct PackageResults {
    name: String,
    records: Vec<ProtoEnvelopeBody>,
}

#[debug_handler]
async fn fetch(
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
