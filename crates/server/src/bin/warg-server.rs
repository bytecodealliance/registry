use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::{net::SocketAddr, path::PathBuf};
use tokio::signal;
use tracing_subscriber::filter::LevelFilter;
use warg_crypto::signing::PrivateKey;
use warg_server::{args::get_opt_content, Config, Server};

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
    #[arg(short, long, env = "VERBOSE", action = clap::ArgAction::Count)]
    verbose: u8,

    /// Address to listen to
    #[arg(short, long, env = "LISTEN", default_value = "127.0.0.1:8090")]
    listen: SocketAddr,

    /// Enable content service, with storage in the given directory
    #[arg(long, env = "CONTENT_DIR")]
    content_dir: Option<PathBuf>,

    /// The data store to use for the server.
    #[arg(long, env = "DATA_STORE", default_value = "memory")]
    data_store: DataStoreKind,

    /// The database connection URL if data-store is set to postgres.
    ///
    /// Prefer using database-url-file, or environment variable variation,
    /// to avoid exposing sensitive information.
    #[cfg(feature = "postgres")]
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// The path to the operator key.
    ///
    /// Takes precedence over database-url.
    #[cfg(feature = "postgres")]
    #[arg(long, env = "DATABASE_URL_FILE")]
    database_url_file: Option<PathBuf>,

    /// The operator key.
    ///
    /// Prefer using warg-operator-key-file, or environment variable variation.
    #[arg(long, env = "WARG_OPERATOR_KEY")]
    warg_operator_key: Option<String>,

    /// The path to the operator key.
    ///
    /// Takes precedence over warg-operator-key.
    #[arg(long, env = "WARG_OPERATOR_KEY_FILE")]
    warg_operator_key_file: Option<PathBuf>,
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

    let operator_key = get_operator_key(&args);

    let mut config = Config::new(operator_key)
        .with_addr(args.listen)
        .with_shutdown(shutdown_signal());

    if let Some(content_dir) = args.content_dir {
        config = config.with_content_dir(content_dir);
    }

    match args.data_store {
        #[cfg(feature = "postgres")]
        DataStoreKind::Postgres => {
            use warg_server::datastore::PostgresDataStore;
            tracing::info!("using postgres data store");
            let database_url =
                get_opt_content("database-url", &args.database_url_file, &args.database_url);
            config = config.with_data_store(PostgresDataStore::new(database_url)?);
        }
        DataStoreKind::Memory => {
            tracing::info!("using memory data store");
        }
    }

    Server::new(config).run().await
}

/// Returns the operator key from the supplied `args` or panics.
///
/// TODO: pull the signing key from the system keyring
fn get_operator_key(args: &Args) -> PrivateKey {
    return match get_opt_content(
        "warg-operator-key",
        &args.warg_operator_key_file,
        &args.warg_operator_key,
    )
    .parse()
    {
        Ok(operator_key) => operator_key,
        Err(why) => panic!("couldn't parse warg-operator-key: {}", why),
    };
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
