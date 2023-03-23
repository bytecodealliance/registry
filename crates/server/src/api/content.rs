use axum::{
    body::boxed,
    debug_handler,
    extract::{BodyStream, OriginalUri, State},
    http::{header::LOCATION, StatusCode},
    response::IntoResponse,
    routing::{get_service, post},
    Router,
};
use futures::StreamExt;
use std::{path::PathBuf, sync::Arc};
use tempfile::NamedTempFile;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tower_http::services::ServeDir;
use warg_crypto::hash::{Digest, Sha256};

#[derive(Debug)]
pub struct ContentConfig {
    pub content_path: PathBuf,
}

impl ContentConfig {
    pub fn build_router(self) -> Router {
        Router::new()
            .route("/", post(upload_content))
            .fallback_service(get_service(ServeDir::new(&self.content_path)).handle_error(
                |err| async move {
                    tracing::error!("ServeDir error: {err}");
                    StatusCode::INTERNAL_SERVER_ERROR
                },
            ))
            .with_state(Arc::new(self))
    }
}

#[derive(Debug, Error)]
pub(crate) enum ContentApiError {
    #[error("failed to allocate temporary file storage")]
    TempFile,
    #[error("failed to read request body: {0}")]
    BodyRead(axum::Error),
    #[error("an error occurred while writing to temporary file storage: {0}")]
    IoError(tokio::io::Error),
    #[error("failed to persist temporary file to content directory")]
    FailedToPersist,
}

impl IntoResponse for ContentApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            Self::TempFile | Self::BodyRead(_) | Self::IoError(_) | Self::FailedToPersist => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        axum::response::Response::builder()
            .status(status)
            .body(boxed(self.to_string()))
            .unwrap()
    }
}

#[debug_handler]
async fn upload_content(
    State(state): State<Arc<ContentConfig>>,
    OriginalUri(orig_uri): OriginalUri,
    mut stream: BodyStream,
) -> Result<impl IntoResponse, ContentApiError> {
    let tmp_path = NamedTempFile::new_in(&state.content_path)
        .map_err(|_| ContentApiError::TempFile)?
        .into_temp_path();

    tracing::debug!("Uploading to {tmp_path:?}");

    let mut hasher = Sha256::new();
    let mut tmp_file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|_| ContentApiError::TempFile)?;
    while let Some(chunk) = stream
        .next()
        .await
        .transpose()
        .map_err(ContentApiError::BodyRead)?
    {
        hasher.update(&chunk);
        tmp_file
            .write_all(&chunk)
            .await
            .map_err(ContentApiError::IoError)?;
    }

    let dest_name = format!("sha256-{:x}", hasher.finalize());
    tmp_path
        .persist(state.content_path.join(&dest_name))
        .map_err(|_| ContentApiError::FailedToPersist)?;

    let location = format!("{}/{}", orig_uri.path().trim_end_matches('/'), dest_name);
    Ok((StatusCode::ACCEPTED, [(LOCATION, location)]))
}
