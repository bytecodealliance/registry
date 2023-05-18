use crate::datastore::MemoryDataStore;
use anyhow::{Context, Result};
use axum::{body::Body, http::Request, Router};
use datastore::DataStore;
use futures::Future;
use monitoring::{LifecycleManager, MonitoringKind};
use services::{CoreService, StopHandle};
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
pub mod monitoring;
mod policy;
pub mod services;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:8090";
const DEFAULT_CHECKPOINT_INTERVAL: Duration = Duration::from_secs(5);

/// The server configuration.
pub struct Config {
    operator_key: Option<PrivateKey>,
    addr: Option<SocketAddr>,
    monitoring_enabled: Option<Vec<MonitoringKind>>,
    monitoring_addr: Option<SocketAddr>,
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
                &self.data_store.as_ref().map(|_| "dyn DataStore"),
            )
            .field("monitoring_enabled", &self.monitoring_enabled)
            .field("monitoring_addr", &self.monitoring_addr)
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
            monitoring_enabled: None,
            monitoring_addr: None,
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

    /// Specify the address for the server to listen on.
    pub fn with_monitoring_enabled(mut self, monitoring_enabled: Vec<MonitoringKind>) -> Self {
        self.monitoring_enabled = Some(monitoring_enabled);
        self
    }

    /// Specify the address for the server to listen on.
    pub fn with_monitoring_addr(mut self, monitoring_addr: impl Into<Option<SocketAddr>>) -> Self {
        self.monitoring_addr = monitoring_addr.into();
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

pub struct Endpoints {
    pub api: SocketAddr,
    pub monitoring: Option<SocketAddr>,
}

/// Represents the warg registry server.
pub struct Server {
    config: Config,
    lifecycle: Arc<LifecycleManager>,
    stop_handle: Option<StopHandle>,
}

impl Server {
    /// Creates a new server with the given configuration.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            lifecycle: Arc::new(LifecycleManager::new(monitoring::Config {
                shutdown_grace_period: Some(Duration::from_secs(5)),
            })),
            stop_handle: None,
        }
    }

    /// Starts the server.
    pub async fn start(&mut self) -> Result<Endpoints> {
        tracing::debug!(
            "using server configuration: {config:?}",
            config = self.config
        );

        let api_addr = self
            .config
            .addr
            .unwrap_or_else(|| DEFAULT_BIND_ADDRESS.parse().unwrap())
            .to_owned();
        let api_listener = TcpListener::bind(api_addr)
            .with_context(|| format!("failed to bind to address `{api_addr}`"))?;
        let local_addr = api_listener.local_addr().unwrap();

        let health_checks_enabled = self
            .config
            .monitoring_enabled
            .as_ref()
            .map(|kinds| kinds.contains(&MonitoringKind::HealthChecks))
            .unwrap_or(false);

        let mut local_monitoring_addr: Option<SocketAddr> = None;
        if health_checks_enabled {
            if let Some(monitoring_addr) = self.config.monitoring_addr.as_ref() {
                let monitoring_listener = TcpListener::bind(monitoring_addr.to_owned())
                    .with_context(|| {
                        format!("failed to bind health_checks to address `{monitoring_addr}`")
                    })
                    .unwrap();
                local_monitoring_addr = Some(monitoring_listener.local_addr().unwrap());
                let monitoring_server = axum::Server::from_tcp(monitoring_listener)
                    .unwrap()
                    .serve(self.lifecycle.health_checks_router().into_make_service());
                tokio::spawn(async move {
                    tracing::info!(
                        "monitoring server on {addr}",
                        addr = local_monitoring_addr.unwrap()
                    );
                    _ = monitoring_server.await;
                    tracing::info!("monitoring server shut down");
                });
            }
        }

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
        self.stop_handle = Some(handle);

        let mut api_router = Router::new().merge(Self::create_router(
            format!("http://{addr}", addr = local_addr),
            self.config.content_dir.take(),
            core,
        )?);

        if health_checks_enabled && local_monitoring_addr.is_none() {
            api_router = api_router.merge(self.lifecycle.health_checks_router());
        }

        let api_lifecycle = self.lifecycle.clone();
        let api_server = axum::Server::from_tcp(api_listener)
            .unwrap()
            .serve(api_router.into_make_service())
            .with_graceful_shutdown(async move {
                api_lifecycle.drain_signal().await;
            });
        tokio::spawn(async move {
            tracing::info!("server listening on {local_addr}");
            _ = api_server.await;
            tracing::info!("server shut down");
        });

        // NOTE: If warmup needed, set live first, do warmup, and then set ready.
        self.lifecycle.set_ready().await?;

        // Set shutdown sequence whether nor not it will be graceful.
        if let Some(shutdown) = self.config.shutdown.take() {
            tracing::debug!("server is running with a shutdown signal");
            let lifecycle = self.lifecycle.clone();
            tokio::spawn(async move {
                shutdown.await;
                lifecycle.shutdown().await.unwrap();
            });
        } else {
            tracing::debug!("server is running without a shutdown signal");
        }

        Ok(Endpoints {
            api: local_addr,
            monitoring: local_monitoring_addr,
        })
    }

    pub async fn join(&mut self) -> Result<()> {
        self.lifecycle.terminate_signal().await;
        tracing::info!("waiting for core service to stop");
        self.stop_handle.take().unwrap().stop().await;
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
