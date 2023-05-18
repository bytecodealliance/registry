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
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
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
    operator_key: Option<PrivateKey>,
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
            operator_key: Some(operator_key),
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
    endpoints: Option<Endpoints>,
    token: CancellationToken,
    tasks: JoinSet<Result<()>>,
}

/// The bound endpoints for the warg registry server.
#[derive(Clone)]
pub struct Endpoints {
    /// The address of the API endpoint.
    pub api: SocketAddr,
}

impl Server {
    /// Creates a new server with the given configuration.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            token: CancellationToken::new(),
            endpoints: None,
            tasks: JoinSet::new(),
        }
    }

    /// Starts the server and binds its endpoints to the configured addresses.
    ///
    /// Returns the endpoints the server bound to.
    pub async fn start(&mut self) -> Result<Endpoints> {
        assert!(
            self.endpoints.is_none(),
            "cannot start server multiple times"
        );

        tracing::debug!(
            "using server configuration: {config:?}",
            config = self.config
        );

        let addr = self
            .config
            .addr
            .to_owned()
            .unwrap_or_else(|| DEFAULT_BIND_ADDRESS.parse().unwrap());

        tracing::debug!("binding api endpoint to address `{addr}`");
        let listener = TcpListener::bind(addr)
            .with_context(|| format!("failed to bind api endpoint to address `{addr}`"))?;

        let addr = listener
            .local_addr()
            .context("failed to get local address for api endpoint listen socket")?;
        tracing::debug!("api endpoint bound to address `{addr}`");

        let endpoints = Endpoints { api: addr };
        self.endpoints = Some(endpoints.clone());

        let store = self
            .config
            .data_store
            .take()
            .unwrap_or_else(|| Box::<MemoryDataStore>::default());
        let (core, handle) = CoreService::spawn(
            self.config.operator_key.take().unwrap(),
            store,
            self.config
                .checkpoint_interval
                .unwrap_or(DEFAULT_CHECKPOINT_INTERVAL),
        )
        .await?;

        let server = axum::Server::from_tcp(listener)?.serve(
            Self::create_router(
                format!("http://{addr}", addr = endpoints.api),
                self.config.content_dir.take(),
                core,
            )?
            .into_make_service(),
        );

        tracing::info!("api endpoint listening on {addr}", addr = endpoints.api);

        // Shut down core service when token cancelled.
        let token = self.token.clone();
        self.tasks.spawn(async move {
            token.cancelled().await;
            tracing::info!("waiting for core service to stop");
            handle.stop().await;
            Ok(())
        });

        // Shut down server when token cancelled.
        let token: CancellationToken = self.token.clone();
        self.tasks.spawn(async move {
            tracing::info!("waiting for api endpoint to stop");
            server
                .with_graceful_shutdown(async move { token.cancelled().await })
                .await?;
            Ok(())
        });

        // Cancel token if shutdown signal received.
        if let Some(shutdown) = self.config.shutdown.take() {
            tracing::debug!("server is running with a shutdown signal");
            let token = self.token.clone();
            tokio::spawn(async move {
                tracing::info!("waiting for shutdown signal");
                shutdown.await;
                tracing::info!("shutting down server");
                token.cancel();
            });
        } else {
            tracing::debug!("server is running without a shutdown signal");
        }

        Ok(endpoints)
    }

    /// Waits on a started server to shutdown.
    pub async fn join(&mut self) -> Result<()> {
        while (self.tasks.join_next().await).is_some() {}
        tracing::info!("server shutdown complete");
        Ok(())
    }

    /// Starts the server and waits for completion.
    pub async fn run(&mut self) -> Result<()> {
        self.start().await?;
        self.join().await?;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.token.cancel();
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
