use axum::{
    debug_handler,
    extract::{BodyStream, OriginalUri, State},
    http::{header::LOCATION, StatusCode},
    response::IntoResponse,
    routing::{get_service, post},
    Json, Router,
};
use futures::StreamExt;
use std::{path::PathBuf, sync::Arc};
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use tower_http::services::ServeDir;
use warg_api::content::ContentError;
use warg_crypto::hash::{Digest, Sha256};

#[derive(Debug)]
pub struct Config {
    dir: PathBuf,
}

impl Config {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn build_router(self) -> Router {
        Router::new()
            .route("/", post(upload_content))
            .fallback_service(get_service(ServeDir::new(&self.dir)))
            .with_state(Arc::new(self))
    }
}

struct ContentApiError(ContentError);

impl IntoResponse for ContentApiError {
    fn into_response(self) -> axum::response::Response {
        // Currently all responses from the content service are 500s
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        (status, Json(self.0)).into_response()
    }
}

#[debug_handler]
async fn upload_content(
    State(state): State<Arc<Config>>,
    OriginalUri(orig_uri): OriginalUri,
    mut stream: BodyStream,
) -> Result<impl IntoResponse, ContentApiError> {
    let tmp_path = NamedTempFile::new_in(&state.dir)
        .map_err(|_| ContentApiError(ContentError::TempFile))?
        .into_temp_path();

    tracing::debug!("Uploading to {tmp_path:?}");

    let mut hasher = Sha256::new();
    let mut tmp_file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|_| ContentApiError(ContentError::TempFile))?;
    while let Some(chunk) = stream.next().await.transpose().map_err(|e| {
        ContentApiError(ContentError::BodyRead {
            message: e.to_string(),
        })
    })? {
        hasher.update(&chunk);
        tmp_file.write_all(&chunk).await.map_err(|e| {
            ContentApiError(ContentError::IoError {
                message: e.to_string(),
            })
        })?;
    }

    let dest_name = format!("sha256-{:x}", hasher.finalize());
    tmp_path
        .persist(state.dir.join(&dest_name))
        .map_err(|_| ContentApiError(ContentError::FailedToPersist))?;

    let location = format!("{}/{}", orig_uri.path().trim_end_matches('/'), dest_name);
    Ok((StatusCode::CREATED, [(LOCATION, location)]))
}
