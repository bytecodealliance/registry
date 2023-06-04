use crate::{api::create_router, datastore::MemoryDataStore};
use anyhow::{bail, Context, Result};
use datastore::DataStore;
use futures::Future;
use policy::content::ContentPolicy;
use services::CoreService;
use std::{
    collections::{HashMap, HashSet},
    fs,
    net::{SocketAddr, TcpListener},
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    time::Duration,
};
use warg_crypto::signing::{KeyID, PrivateKey};

pub mod api;
pub mod args;
pub mod datastore;
pub mod policy;
pub mod services;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:8090";
const DEFAULT_CHECKPOINT_INTERVAL: Duration = Duration::from_secs(5);

fn is_kebab_case(s: &str) -> bool {
    let mut lower = false;
    let mut upper = false;
    for c in s.chars() {
        match c {
            'a'..='z' if !lower && !upper => lower = true,
            'A'..='Z' if !lower && !upper => upper = true,
            'a'..='z' if lower => continue,
            'A'..='Z' if upper => continue,
            '0'..='9' if lower || upper => continue,
            '-' if lower || upper => {
                lower = false;
                upper = false;
                continue;
            }
            _ => return false,
        }
    }

    !s.is_empty() && !s.ends_with('-')
}

/// The server configuration.
pub struct Config {
    operator_key: PrivateKey,
    addr: Option<SocketAddr>,
    data_store: Option<Box<dyn datastore::DataStore>>,
    content_dir: PathBuf,
    shutdown: Option<Pin<Box<dyn Future<Output = ()> + Send + Sync>>>,
    checkpoint_interval: Option<Duration>,
    content_policy: Option<Arc<dyn ContentPolicy>>,
    authorized_keys: Option<HashMap<String, HashSet<KeyID>>>,
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
            .field("authorized_keys", &self.authorized_keys)
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
            shutdown: None,
            checkpoint_interval: None,
            content_policy: None,
            authorized_keys: Some(Default::default()),
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

    /// Sets an authorized key for a particular namespace.
    pub fn with_authorized_key(mut self, namespace: impl Into<String>, key: KeyID) -> Result<Self> {
        let namespace = namespace.into();
        if !is_kebab_case(&namespace) {
            bail!("namespace `{namespace}` is not a legal kebab-case identifier");
        }

        self.authorized_keys
            .get_or_insert_with(Default::default)
            .entry(namespace)
            .or_default()
            .insert(key);
        Ok(self)
    }

    /// Sets the configuration to allow any key to publish a package record.
    ///
    /// This will clear any previously set authorized keys.
    pub fn with_no_authorization(mut self) -> Self {
        self.authorized_keys = None;
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

        let base_url = format!("http://{addr}", addr = self.config.addr.unwrap());
        let server = axum::Server::from_tcp(listener)?.serve(
            create_router(
                base_url,
                core,
                temp_dir,
                files_dir,
                self.config.content_policy,
                self.config.authorized_keys,
            )
            .into_make_service(),
        );

        tracing::info!("listening on {addr}", addr = self.config.addr.unwrap());

        if let Some(shutdown) = self.config.shutdown {
            tracing::debug!("server is running with a shutdown signal");
            server.with_graceful_shutdown(shutdown).await?;
        } else {
            tracing::debug!("server is running without a shutdown signal");
            server.await?;
        }

        tracing::info!("waiting for core service to stop");
        handle.stop().await;
        tracing::info!("server shutdown complete");

        Ok(())
    }
}
