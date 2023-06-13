use crate::{
    policy::{content::ContentPolicy, record::RecordPolicy},
    services::CoreService,
};
use anyhow::Result;
use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection},
        FromRequest, FromRequestParts,
    },
    http::StatusCode,
    response::IntoResponse,
    Router,
};
use serde::{Serialize, Serializer};
use std::{path::PathBuf, sync::Arc};
use url::Url;

pub mod fetch;
pub mod package;
pub mod proof;

/// An extractor that wraps the JSON extractor of Axum.
///
/// This extractor returns an API error on rejection.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(Error))]
pub struct Json<T>(T);

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        axum::Json(self.0).into_response()
    }
}

fn serialize_status<S>(status: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u16(status.as_u16())
}

/// Represents a generic error from the API.
#[derive(Serialize, Debug)]
pub struct Error {
    #[serde(serialize_with = "serialize_status")]
    status: StatusCode,
    message: String,
}

impl From<JsonRejection> for Error {
    fn from(rejection: JsonRejection) -> Self {
        Self {
            status: rejection.status(),
            message: rejection.body_text(),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (self.status, axum::Json(self)).into_response()
    }
}

/// An extractor that wraps the path extractor of Axum.
///
/// This extractor returns an API error on rejection.
#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(Error))]
pub struct Path<T>(T);

impl From<PathRejection> for Error {
    fn from(rejection: PathRejection) -> Self {
        Self {
            status: rejection.status(),
            message: rejection.body_text(),
        }
    }
}

pub async fn not_found() -> impl IntoResponse {
    Error {
        status: StatusCode::NOT_FOUND,
        message: "the requested resource was not found".to_string(),
    }
}

pub fn create_router(
    content_base_url: Url,
    core: Arc<CoreService>,
    temp_dir: PathBuf,
    files_dir: PathBuf,
    content_policy: Option<Arc<dyn ContentPolicy>>,
    record_policy: Option<Arc<dyn RecordPolicy>>,
) -> Router {
    let proof_config = proof::Config::new(core.log_data().clone(), core.map_data().clone());
    let package_config = package::Config::new(
        core.clone(),
        content_base_url,
        files_dir,
        temp_dir,
        content_policy,
        record_policy,
    );
    let fetch_config = fetch::Config::new(core);

    Router::new()
        .nest("/package", package_config.into_router())
        .nest("/fetch", fetch_config.into_router())
        .nest("/proof", proof_config.into_router())
        .fallback(not_found)
}
