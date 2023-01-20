mod api;
mod policy;
mod services;

use std::{fmt, path::PathBuf, sync::Arc};

use anyhow::Result;
use axum::{http::StatusCode, response::IntoResponse, Router};

use api::content::ContentConfig;
use services::{
    core::{CoreService, State},
    transparency,
};
use tokio::sync::mpsc;
use warg_protocol::signing::PrivateKey;

pub struct Config {
    base_url: String,
    signing_key: PrivateKey,
    content: Option<ContentConfig>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("signing_key", &"<REDACTED>")
            .field("content", &self.content)
            .finish()
    }
}

impl Config {
    pub fn new(base_url: String, signing_key: PrivateKey) -> Self {
        Config {
            base_url,
            signing_key,
            content: None,
        }
    }

    pub fn enable_content_service(&mut self, content_path: PathBuf) -> &mut Self {
        self.content = Some(ContentConfig { content_path });
        self
    }

    pub fn build_router(self) -> Result<Router> {
        let (transparency_tx, transparency_rx) = mpsc::channel(4);

        let input = transparency::Input {
            log: transparency::VerifiableLog::default(),
            map: transparency::VerifiableMap::default(),
            private_key: self.signing_key,
            log_rx: transparency_rx,
        };

        let initial_state = State::default();
        let core = Arc::new(CoreService::new(initial_state, transparency_tx));

        let mut router: Router = Router::new();
        if let Some(upload) = self.content {
            router = router.nest("/content", upload.build_router()?);
        }

        let package_config = api::package::Config::new(core.clone(), self.base_url.clone());

        router = router.nest("/package", package_config.build_router());

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
