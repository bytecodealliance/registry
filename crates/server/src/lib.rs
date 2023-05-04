use crate::datastore::MemoryDataStore;
use anyhow::{Context, Result};
use axum::{body::Body, http::Request, Router};
use datastore::DataStore;
use futures::Future;
use services::CoreService;
use std::{
    fs,
    net::{SocketAddr, TcpListener},
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    time::Duration,
};
use tower_http::{
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{Level, Span};
use warg_crypto::signing::PrivateKey;

pub mod api;
pub mod args;
pub mod datastore;
mod policy;
pub mod services;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:8090";
const DEFAULT_CHECKPOINT_INTERVAL: Duration = Duration::from_secs(5);

/// The server configuration.
pub struct Config {
    operator_key: PrivateKey,
    addr: Option<SocketAddr>,
    data_store: Option<Box<dyn datastore::DataStore>>,
    content_dir: Option<PathBuf>,
    shutdown: Option<Pin<Box<dyn Future<Output = ()> + Send + Sync>>>,
    checkpoint_interval: Option<Duration>,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("operator_key", &"<redacted>")
            .field("addr", &self.addr)
            .field(
                "data_store",
                &self.shutdown.as_ref().map(|_| "dyn DataStore"),
            )
            .field("content", &self.content_dir)
            .field("shutdown", &self.shutdown.as_ref().map(|_| "dyn Future"))
            .field("checkpoint_interval", &self.checkpoint_interval)
            .finish()
    }
}

impl Config {
    /// Creates a new server configuration.
    pub fn new(operator_key: PrivateKey) -> Self {
        Self {
            operator_key,
            addr: None,
            data_store: None,
            content_dir: None,
            shutdown: None,
            checkpoint_interval: None,
        }
    }

    /// Specify the address for the server to listen on.
    pub fn with_addr(mut self, addr: impl Into<SocketAddr>) -> Self {
        self.addr = Some(addr.into());
        self
    }

    /// Specify the data store to use.
    ///
    /// If this is not specified, the server will use an in-memory data store.
    pub fn with_data_store(mut self, store: impl DataStore + 'static) -> Self {
        self.data_store = Some(Box::new(store));
        self
    }

    /// Specify the data store to use via a boxed data store.
    ///
    /// If this is not specified, the server will use an in-memory data store.
    pub fn with_boxed_data_store(mut self, store: Box<dyn DataStore>) -> Self {
        self.data_store = Some(store);
        self
    }

    /// Specify the path to the directory where content will be stored.
    ///
    /// If the directory does not exist, it will be created.
    ///
    /// This enables the content API in the server.
    pub fn with_content_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.content_dir = Some(path.into());
        self
    }

    /// Specifies the future to wait on to shutdown the server.
    ///
    /// If the future completes, the server will initiate a graceful shutdown.
    pub fn with_shutdown(
        mut self,
        shutdown: impl Future<Output = ()> + Send + Sync + 'static,
    ) -> Self {
        self.shutdown = Some(Box::pin(shutdown));
        self
    }

    /// Sets the checkpoint interval to use for the server.
    pub fn with_checkpoint_interval(mut self, interval: Duration) -> Self {
        self.checkpoint_interval = Some(interval);
        self
    }
}

/// Represents the warg registry server.
pub struct Server {
    config: Config,
    listener: Option<TcpListener>,
}

impl Server {
    /// Creates a new server with the given configuration.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            listener: None,
        }
    }

    /// Binds the server to the configured address.
    ///
    /// Returns the address the server bound to.
    pub fn bind(&mut self) -> Result<SocketAddr> {
        let addr = self
            .config
            .addr
            .unwrap_or_else(|| DEFAULT_BIND_ADDRESS.parse().unwrap());

        tracing::debug!("binding server to address `{addr}`");
        let listener = TcpListener::bind(addr)
            .with_context(|| format!("failed to bind to address `{addr}`"))?;

        let addr = listener
            .local_addr()
            .context("failed to get local address for listen socket")?;

        tracing::debug!("server bound to address `{addr}`");
        self.config.addr = Some(addr);
        self.listener = Some(listener);
        Ok(addr)
    }

    /// Runs the server.
    pub async fn run(mut self) -> Result<()> {
        if self.listener.is_none() {
            self.bind()?;
        }

        let listener = self.listener.unwrap();

        tracing::debug!(
            "using server configuration: {config:?}",
            config = self.config
        );

        let store = self
            .config
            .data_store
            .unwrap_or_else(|| Box::<MemoryDataStore>::default());
        let (core, handle) = CoreService::spawn(
            self.config.operator_key,
            store,
            self.config
                .checkpoint_interval
                .unwrap_or(DEFAULT_CHECKPOINT_INTERVAL),
        )
        .await?;

        let server = axum::Server::from_tcp(listener)?.serve(
            Self::create_router(
                format!("http://{addr}", addr = self.config.addr.unwrap()),
                self.config.content_dir,
                core,
            )?
            .into_make_service(),
        );

        tracing::info!("listening on {addr}", addr = self.config.addr.unwrap());

        if let Some(shutdown) = self.config.shutdown {
            tracing::debug!("server is running with a shutdown signal");
            server
                .with_graceful_shutdown(async move { shutdown.await })
                .await?;
        } else {
            tracing::debug!("server is running without a shutdown signal");
            server.await?;
        }

        tracing::info!("waiting for core service to stop");
        handle.stop().await;
        tracing::info!("server shutdown complete");

        Ok(())
    }

    fn create_router(
        base_url: String,
        content_dir: Option<PathBuf>,
        core: Arc<CoreService>,
    ) -> Result<Router> {
        let proof_config =
            api::proof::Config::new(core.log_data().clone(), core.map_data().clone());
        let package_config = api::package::Config::new(core.clone(), base_url);
        let fetch_config = api::fetch::Config::new(core);

        let mut router = Router::new();
        if let Some(content_dir) = content_dir {
            fs::create_dir_all(&content_dir).with_context(|| {
                format!(
                    "failed to create content directory `{path}`",
                    path = content_dir.display()
                )
            })?;

            let config = api::content::Config::new(content_dir);
            router = router.nest("/content", config.build_router());
        }

        Ok(router
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
            ))
    }
}
