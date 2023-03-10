use crate::AnyError;
use anyhow::Result;
use axum::{
    body::Body,
    debug_handler,
    extract::{BodyStream, OriginalUri, Path, State},
    http::{header::LOCATION, Request, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use futures::StreamExt;
use std::{path::PathBuf, str::FromStr, sync::Arc};
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use tower::ServiceExt;
use tower_http::services::ServeDir;
use warg_crypto::hash::{Digest, DynHash, Sha256};

#[derive(Debug)]
pub struct ContentConfig {
    pub content_path: PathBuf,
}

impl ContentConfig {
    pub fn build_router(self) -> Result<Router> {
        Ok(Router::new()
            .route("/:content_id", get(get_content))
            .route("/", post(upload_content))
            .with_state(Arc::new(self)))
    }
}

#[debug_handler]
async fn get_content(
    State(state): State<Arc<ContentConfig>>,
    Path(content_id): Path<String>,
    mut req: Request<Body>,
) -> Result<impl IntoResponse, AnyError> {
    let content_id = DynHash::from_str(&content_id)?;
    *req.uri_mut() = format!("/{content_id}").replace(':', "-").parse().unwrap();
    Ok(ServeDir::new(&state.content_path).oneshot(req).await?)
}

#[debug_handler]
async fn upload_content(
    State(state): State<Arc<ContentConfig>>,
    OriginalUri(orig_uri): OriginalUri,
    mut stream: BodyStream,
) -> Result<impl IntoResponse, AnyError> {
    let tmp_path = NamedTempFile::new_in(&state.content_path)?.into_temp_path();
    tracing::debug!("uploading content to `{path}`", path = tmp_path.display());

    let mut hasher = Sha256::new();
    let mut tmp_file = tokio::fs::File::create(&tmp_path).await?;
    while let Some(chunk) = stream.next().await.transpose()? {
        hasher.update(&chunk);
        tmp_file.write_all(&chunk).await?;
    }

    let digest = hasher.finalize();
    tmp_path.persist(state.content_path.join(format!("sha256-{digest:x}")))?;

    Ok((
        StatusCode::CREATED,
        [(LOCATION, format!("{orig_uri}/sha256:{digest:x}"))],
    ))
}
