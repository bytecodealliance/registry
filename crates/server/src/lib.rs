use api::content::ContentConfig;
use axum::{body::Body, http::Request, Router};
use std::{fmt, path::PathBuf};
use tower_http::{
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{Level, Span};
use warg_crypto::signing::PrivateKey;

pub mod api;
mod policy;
pub mod services;

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

    pub fn build_router(self) -> Router {
        let mut router = Router::new();
        if let Some(upload) = self.content {
            router = router.nest("/content", upload.build_router());
        }

        let (core, data) = services::init(self.signing_key);
        let package_config = api::package::Config::new(core.clone(), self.base_url.clone());
        let fetch_config = api::fetch::Config::new(core);
        let proof_config = api::proof::Config::new(data.log_data, data.map_data);

        router
            .nest("/package", package_config.build_router())
            .nest("/fetch", fetch_config.build_router())
            .nest("/proof", proof_config.build_router())
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
