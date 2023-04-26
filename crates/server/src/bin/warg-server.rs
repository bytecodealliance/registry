use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::{
    fs::File,
    io::prelude::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
};
use tokio::signal;
use tracing_subscriber::filter::LevelFilter;
use warg_crypto::signing::PrivateKey;
use warg_server::{Config, Server};

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
    // TODO: Add --warg-operator-key[-file] and others cli options
    let operator_key: PrivateKey = match std::env::var("WARG_OPERATOR_KEY_FILE") {
        Ok(path_env) => {
            let path = Path::new(&path_env);
            let mut file = match File::open(path) {
                Err(why) => panic!("couldn't open {}: {}", path.display(), why),
                Ok(file) => file,
            };
            let mut s = String::new();
            match file.read_to_string(&mut s) {
                Err(why) => panic!("couldn't read {}: {}", path.display(), why),
                Ok(_) => s,
            }
        },
        Err(_) => {
            match std::env::var("WARG_OPERATOR_KEY") {
                Ok(key) => key,
                Err(_) => panic!("either WARG_OPERATOR_KEY_FILE or WARG_OPERATOR_KEY environment variable needs to be set"),
            }
        },
    }.parse()?;
    let mut config = Config::new(operator_key)
        .with_addr(args.listen)
        .with_shutdown(shutdown_signal());

    if let Some(content_dir) = args.content_dir {
        config = config.with_content_dir(content_dir);
    }

    match args.data_store {
        #[cfg(feature = "postgres")]
        DataStoreKind::Postgres => {
            use anyhow::Context;
            use warg_server::datastore::PostgresDataStore;
            tracing::debug!("using PostgreSQL data store");
            config = config.with_data_store(PostgresDataStore::new(
                std::env::var("DATABASE_URL").context(
                    "failed to get the PostgreSQL database URL from the `DATABASE_URL` environment variable",
                )?,
            )?);
        }
        DataStoreKind::Memory => {
            tracing::debug!("using default data store");
        }
    }

    Server::new(config).run().await
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
