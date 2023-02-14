use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use axum::{
    debug_handler,
    extract::{BodyStream, OriginalUri, State},
    http::{header::LOCATION, StatusCode},
    response::IntoResponse,
    routing::{get_service, post},
    Router,
};
use futures::StreamExt;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use tower_http::services::ServeDir;
use warg_crypto::hash::{Digest, Sha256};

use crate::AnyError;

#[derive(Debug)]
pub struct ContentConfig {
    pub content_path: PathBuf,
}

impl ContentConfig {
    pub fn build_router(self) -> Result<Router> {
        Ok(Router::new()
            .route("/", post(upload_content))
            .fallback_service(get_service(ServeDir::new(&self.content_path)).handle_error(
                |err| async move {
                    tracing::error!("ServeDir error: {err}");
                    StatusCode::INTERNAL_SERVER_ERROR
                },
            ))
            .with_state(Arc::new(self)))
    }
}

#[debug_handler]
async fn upload_content(
    State(state): State<Arc<ContentConfig>>,
    OriginalUri(orig_uri): OriginalUri,
    mut stream: BodyStream,
) -> Result<impl IntoResponse, AnyError> {
    let tmp_path = NamedTempFile::new_in(&state.content_path)?.into_temp_path();
    tracing::debug!("Uploading to {tmp_path:?}");

    let mut hasher = Sha256::new();
    let mut tmp_file = tokio::fs::File::create(&tmp_path).await?;
    while let Some(chunk) = stream.next().await.transpose()? {
        hasher.update(&chunk);
        tmp_file.write_all(&chunk).await?;
    }

    let dest_name = format!("sha256-{:x}", hasher.finalize());
    tmp_path.persist(state.content_path.join(&dest_name))?;

    let location = format!("{}/{}", orig_uri.path().trim_end_matches('/'), dest_name);
    Ok((StatusCode::OK, [(LOCATION, location)]))
}
