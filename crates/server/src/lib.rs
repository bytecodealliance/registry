use crate::{api::create_router, datastore::MemoryDataStore};
use anyhow::{Context, Result};
use axum::Router;
use datastore::DataStore;
use futures::Future;
use policy::{content::ContentPolicy, record::RecordPolicy};
use services::CoreService;
use std::{
    fs,
    net::{SocketAddr, TcpListener},
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    time::Duration,
};
use tokio::task::JoinHandle;
use url::Url;
use warg_crypto::signing::PrivateKey;

pub mod api;
pub mod args;
pub mod datastore;
pub mod policy;
pub mod services;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:8090";
const DEFAULT_CHECKPOINT_INTERVAL: Duration = Duration::from_secs(5);

type ShutdownFut = Pin<Box<dyn Future<Output = ()> + Send + Sync>>;

/// The server configuration.
pub struct Config {
    operator_key: PrivateKey,
    addr: Option<SocketAddr>,
    data_store: Option<Box<dyn DataStore>>,
    content_dir: PathBuf,
    content_base_url: Option<Url>,
    shutdown: Option<ShutdownFut>,
    checkpoint_interval: Option<Duration>,
    content_policy: Option<Arc<dyn ContentPolicy>>,
    record_policy: Option<Arc<dyn RecordPolicy>>,
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
            .field("content_dir", &self.content_dir)
            .field("shutdown", &self.shutdown.as_ref().map(|_| "dyn Future"))
            .field("checkpoint_interval", &self.checkpoint_interval)
            .field(
                "content_policy",
                &self.content_policy.as_ref().map(|_| "dyn ContentPolicy"),
            )
            .field(
                "record_policy",
                &self.record_policy.as_ref().map(|_| "dyn RecordPolicy"),
            )
            .finish()
    }
}

impl Config {
    /// Creates a new server configuration.
    pub fn new(operator_key: PrivateKey, content_dir: PathBuf) -> Self {
        Self {
            operator_key,
            addr: None,
            data_store: None,
            content_dir,
            content_base_url: None,
            shutdown: None,
            checkpoint_interval: None,
            content_policy: None,
            record_policy: None,
        }
    }

    /// Specify the address for the server to listen on.
    pub fn with_addr(mut self, addr: impl Into<SocketAddr>) -> Self {
        self.addr = Some(addr.into());
        self
    }

    /// Specify the content base URL to use.
    ///
    /// If not set, the content base URL will be derived from the server address.
    pub fn with_content_base_url(mut self, url: Url) -> Self {
        self.content_base_url = Some(url);
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

    /// Sets the content policy to use for the server.
    pub fn with_content_policy(mut self, policy: impl ContentPolicy + 'static) -> Self {
        self.content_policy = Some(Arc::new(policy));
        self
    }

    /// Sets the record policy to use for the server.
    pub fn with_record_policy(mut self, policy: impl RecordPolicy + 'static) -> Self {
        self.record_policy = Some(Arc::new(policy));
        self
    }
}

/// Represents the warg registry server.
pub struct Server {
    config: Config,
}

impl Server {
    /// Creates a new server with the given configuration.
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Initializes the server and starts serving.
    ///
    /// Equivalent to calling [`Server::initialize`] followed by
    /// [`InitializedServer::serve`].
    pub async fn run(self) -> Result<()> {
        self.initialize().await?.serve().await
    }

    /// Initializes the server's internal state, background task(s), and
    /// listening socket, returning an [`InitializedServer`]. To actually begin
    /// serving, call [`InitializedServer::serve`].
    ///
    /// Useful for tests that need full initialization before running.
    pub async fn initialize(self) -> Result<InitializedServer> {
        let addr = self
            .config
            .addr
            .unwrap_or_else(|| DEFAULT_BIND_ADDRESS.parse().unwrap());

        tracing::debug!("binding server to address `{addr}`");
        let listener = TcpListener::bind(addr)
            .with_context(|| format!("failed to bind to address `{addr}`"))?;
        let addr = listener.local_addr()?;

        tracing::debug!(
            "using server configuration: {config:?}",
            config = self.config
        );

        let store = self
            .config
            .data_store
            .unwrap_or_else(|| Box::<MemoryDataStore>::default());
        let (core, core_handle) = CoreService::start(
            self.config.operator_key,
            store,
            self.config
                .checkpoint_interval
                .unwrap_or(DEFAULT_CHECKPOINT_INTERVAL),
        )
        .await?;

        let temp_dir = self.config.content_dir.join("tmp");
        fs::create_dir_all(&temp_dir).with_context(|| {
            format!(
                "failed to create content temp directory `{path}`",
                path = temp_dir.display()
            )
        })?;

        let files_dir = self.config.content_dir.join("files");
        fs::create_dir_all(&files_dir).with_context(|| {
            format!(
                "failed to create content files directory `{path}`",
                path = files_dir.display()
            )
        })?;

        let content_base_url = self
            .config
            .content_base_url
            .unwrap_or_else(|| Url::parse(&format!("http://{addr}")).unwrap());

        let router = create_router(
            content_base_url,
            core,
            temp_dir,
            files_dir,
            self.config.content_policy,
            self.config.record_policy,
        );

        Ok(InitializedServer {
            listener,
            router,
            core_handle,
            shutdown: self.config.shutdown,
        })
    }
}

/// Represents an initialized warg registry server.
pub struct InitializedServer {
    listener: TcpListener,
    router: Router,
    core_handle: JoinHandle<()>,
    shutdown: Option<ShutdownFut>,
}

impl InitializedServer {
    /// Returns the listening address of the server. If a random listening
    /// port was requested (i.e. `:0`), this returns the actual bound port.
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Serves the server's services. On server shutdown, awaits completion of
    /// background task(s) before returning.
    pub async fn serve(self) -> Result<()> {
        let addr = self.local_addr()?;

        let server = axum::Server::from_tcp(self.listener)?.serve(self.router.into_make_service());

        tracing::info!("listening on {addr}");

        if let Some(shutdown) = self.shutdown {
            tracing::debug!("server is running with a shutdown signal");
            server.with_graceful_shutdown(shutdown).await?;
        } else {
            tracing::debug!("server is running without a shutdown signal");
            server.await?;
        }

        tracing::info!("waiting for core service to stop");
        self.core_handle.await?;

        tracing::info!("server shutdown complete");
        Ok(())
    }
}
