use super::{Json, Path};
use axum::{
    debug_handler, extract::State, http::StatusCode, response::IntoResponse, routing::get, Router,
};
use std::{collections::HashMap, path::PathBuf};
use url::Url;
use warg_api::v1::content::{ContentError, ContentSource, ContentSourcesResponse};
use warg_crypto::hash::AnyHash;

#[derive(Clone)]
pub struct Config {
    content_base_url: Url,
    files_dir: PathBuf,
}

impl Config {
    pub fn new(content_base_url: Url, files_dir: PathBuf) -> Self {
        Self {
            content_base_url,
            files_dir,
        }
    }

    pub fn into_router(self) -> Router {
        Router::new()
            .route("/:digest", get(get_content))
            .with_state(self)
    }

    fn content_present(&self, digest: &AnyHash) -> bool {
        self.content_path(digest).is_file()
    }

    fn content_file_name(&self, digest: &AnyHash) -> String {
        digest.to_string().replace(':', "-")
    }

    fn content_path(&self, digest: &AnyHash) -> PathBuf {
        self.files_dir.join(self.content_file_name(digest))
    }

    fn content_url(&self, digest: &AnyHash) -> String {
        self.content_base_url
            .join("content/")
            .unwrap()
            .join(&self.content_file_name(digest))
            .unwrap()
            .to_string()
    }
}

struct ContentApiError(ContentError);

impl IntoResponse for ContentApiError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::from_u16(self.0.status()).unwrap(), Json(self.0)).into_response()
    }
}

#[debug_handler]
async fn get_content(
    State(config): State<Config>,
    Path(digest): Path<AnyHash>,
) -> Result<Json<ContentSourcesResponse>, ContentApiError> {
    if !config.content_present(&digest) {
        return Err(ContentApiError(ContentError::ContentDigestNotFound(digest)));
    }

    let mut content_sources = HashMap::with_capacity(1);
    let url = config.content_url(&digest);
    content_sources.insert(
        digest,
        vec![ContentSource::HttpGet {
            url,
            headers: HashMap::new(),
            supports_range_header: false,
            size: None,
        }],
    );

    Ok(Json(ContentSourcesResponse { content_sources }))
}
