use crate::{
    policy::{content::ContentPolicy, record::RecordPolicy},
    services::CoreService,
};
use axum::{body::Body, http::Request, Router};
use std::{path::PathBuf, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{Level, Span};
use url::Url;

pub mod v1;

/// Creates the router for the API.
pub fn create_router(
    content_base_url: Url,
    core: Arc<CoreService>,
    temp_dir: PathBuf,
    files_dir: PathBuf,
    content_policy: Option<Arc<dyn ContentPolicy>>,
    record_policy: Option<Arc<dyn RecordPolicy>>,
) -> Router {
    Router::new()
        .nest(
            "/v1",
            v1::create_router(
                content_base_url,
                core,
                temp_dir,
                files_dir.clone(),
                content_policy,
                record_policy,
            ),
        )
        .nest_service("/content", ServeDir::new(files_dir))
        .layer(
            ServiceBuilder::new()
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
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                        .allow_headers([
                            axum::http::header::CONTENT_TYPE,
                            axum::http::header::ACCEPT,
                        ]),
                ),
        )
}
