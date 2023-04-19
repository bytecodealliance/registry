use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::{net::SocketAddr, path::PathBuf};
use tokio::signal;
use tracing_subscriber::filter::LevelFilter;
use warg_server::{
    datastore::{DataStore, MemoryDataStore},
    services::CoreService,
    Config,
};

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DataStoreKind {
    #[cfg(feature = "postgres")]
    Postgres,
    #[default]
    Memory,
}

#[derive(Parser, Debug)]
struct Args {
    /// Use verbose output
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Address to listen to
    #[arg(short, long, default_value = "127.0.0.1:8090")]
    listen: SocketAddr,

    /// Enable content service, with storage in the given directory
    #[arg(long)]
    content_dir: Option<PathBuf>,

    /// The data store to use for the server.
    #[arg(long, default_value = "memory")]
    data_store: DataStoreKind,
}

impl Args {
    fn init_tracing(&self) {
        let level_filter = match self.verbose {
            0 => LevelFilter::INFO,
            1 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        };
        tracing_subscriber::fmt()
            .with_max_level(level_filter)
            .init();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    args.init_tracing();
    tracing::debug!("args: {args:?}");

    // TODO: pull the signing key from the system keyring
    let demo_operator_key = std::env::var("WARG_DEMO_OPERATOR_KEY")?;
    let signing_key = demo_operator_key.parse()?;

    let base_url = format!("http://{}", args.listen);
    let mut config = Config::new(base_url);
    if let Some(path) = args.content_dir {
        config.enable_content_service(path)?;
    }
    tracing::debug!("config: {config:?}");

    let store: Box<dyn DataStore> = match args.data_store {
        #[cfg(feature = "postgres")]
        DataStoreKind::Postgres => {
            use anyhow::Context;
            tracing::debug!("using PostgreSQL data store");
            Box::new(warg_server::datastore::PostgresBackend::new(
                std::env::var("DATABASE_URL").context(
                    "failed to get the database URL from the `DATABASE_URL` environment variable",
                )?,
            )?)
        }
        DataStoreKind::Memory => {
            tracing::debug!("using in-memory data store");
            Box::<MemoryDataStore>::default()
        }
    };

    let (core, handle) = CoreService::spawn(signing_key, store).await?;

    tracing::info!("listening on {:?}", args.listen);
    axum::Server::try_bind(&args.listen)?
        .serve(config.into_router(core).into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("waiting for core service to stop");
    handle.stop().await;
    tracing::info!("shutdown complete");

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");

        tracing::info!("starting shutdown (SIGINT)");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;

        tracing::info!("starting shutdown (SIGTERM)");
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
