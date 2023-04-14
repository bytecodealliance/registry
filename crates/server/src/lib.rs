use anyhow::{Context, Result};
use api::content::ContentConfig;
use axum::{body::Body, http::Request, Router};
use services::CoreService;
use std::{fs, path::PathBuf, sync::Arc};
use tower_http::{
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{Level, Span};

pub mod api;
pub mod datastore;
mod policy;
pub mod services;

#[derive(Debug)]
pub struct Config {
    base_url: String,
    content: Option<ContentConfig>,
}

impl Config {
    pub fn new(base_url: String) -> Self {
        Config {
            base_url,
            content: None,
        }
    }

    pub fn enable_content_service(&mut self, content_path: PathBuf) -> Result<&mut Self> {
        fs::create_dir_all(&content_path).with_context(|| {
            format!(
                "failed to create content directory `{path}`",
                path = content_path.display()
            )
        })?;

        self.content = Some(ContentConfig { content_path });
        Ok(self)
    }

    pub fn into_router(self, core: Arc<CoreService>) -> Router {
        let proof_config =
            api::proof::Config::new(core.log_data().clone(), core.map_data().clone());
        let package_config = api::package::Config::new(core.clone(), self.base_url.clone());
        let fetch_config = api::fetch::Config::new(core);

        let mut router = Router::new();
        if let Some(upload) = self.content {
            router = router.nest("/content", upload.build_router());
        }

        router
            .nest("/package", package_config.into_router())
            .nest("/fetch", fetch_config.into_router())
            .nest("/proof", proof_config.into_router())
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().include_headers(true))
                    .on_request(|request: &Request<Body>, _span: &Span| {
                        tracing::info!("starting {} {}", request.method(), request.uri().path())
                    })
                    .on_response(
                        DefaultOnResponse::new()
                            .level(Level::INFO)
                            .latency_unit(LatencyUnit::Micros),
                    ),
            )
    }
}
