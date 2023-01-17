mod api;
mod policy;
mod services;

use std::path::PathBuf;

use anyhow::Result;
use axum::{http::StatusCode, response::IntoResponse, Router};

use api::content::ContentConfig;

#[derive(Debug, Default)]
pub struct Config {
    content: Option<ContentConfig>,
}

impl Config {
    pub fn enable_content_service(&mut self, content_path: PathBuf) -> &mut Self {
        self.content = Some(ContentConfig { content_path });
        self
    }

    pub fn build_router(self) -> Result<Router> {
        let mut router = Router::new();
        if let Some(upload) = self.content {
            router = router.nest("/content", upload.build_router()?);
        }
        Ok(router)
    }
}

pub(crate) struct AnyError(anyhow::Error);

impl<E: Into<anyhow::Error>> From<E> for AnyError {
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl IntoResponse for AnyError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("Handler failed: {}", self.0);
        // TODO: don't return arbitrary errors to clients
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error: {}", self.0),
        )
            .into_response()
    }
}
