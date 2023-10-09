use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use secrecy::SecretString;
use std::{net::SocketAddr, path::PathBuf};
use tokio::signal;
use tracing_subscriber::filter::LevelFilter;
use url::Url;
use warg_crypto::signing::PrivateKey;
use warg_server::{
    args::get_opt_secret, extractor::metadata, policy::record::AuthorizedKeyPolicy, Config, Server,
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
    #[arg(short, long, env = "WARG_VERBOSE", action = clap::ArgAction::Count)]
    verbose: u8,

    /// Address to listen to
    #[arg(short, long, env = "WARG_LISTEN", default_value = "127.0.0.1:8090")]
    listen: SocketAddr,

    /// The content storage directory to use.
    #[arg(long, env = "WARG_CONTENT_DIR")]
    content_dir: PathBuf,

    /// The base content URL to use; defaults to the server address.
    #[arg(long, env = "WARG_CONTENT_BASE_URL")]
    content_base_url: Option<Url>,

    /// The data store to use for the server.
    #[arg(long, env = "WARG_DATA_STORE", default_value = "memory")]
    data_store: DataStoreKind,

    /// The database connection URL if data-store is set to postgres.
    ///
    /// Prefer using `database-url-file`, or environment variable variation,
    /// to avoid exposing sensitive information.
    #[cfg(feature = "postgres")]
    #[arg(long, env = "WARG_DATABASE_URL")]
    database_url: Option<SecretString>,

    /// The path to the database connection URL file.
    #[cfg(feature = "postgres")]
    #[arg(long, env = "WARG_DATABASE_URL_FILE", conflicts_with = "database_url")]
    database_url_file: Option<PathBuf>,

    /// Run database migrations
    #[cfg(feature = "postgres")]
    #[arg(long)]
    database_run_migrations: bool,

    /// The operator key.
    ///
    /// Prefer using `operator-key-file`, or environment variable variation.
    #[arg(long, env = "WARG_OPERATOR_KEY")]
    operator_key: Option<SecretString>,

    /// The path to the operator key.
    #[arg(long, env = "WARG_OPERATOR_KEY_FILE", conflicts_with = "operator_key")]
    operator_key_file: Option<PathBuf>,

    /// The path to the authorized keys record policy file.
    #[arg(long, env = "WARG_AUTHORIZED_KEYS_FILE")]
    authorized_keys_file: Option<PathBuf>,
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

    let operator_key_str =
        get_opt_secret("operator-key", args.operator_key_file, args.operator_key)?;
    let operator_key =
        PrivateKey::decode(operator_key_str).context("failed to parse operator key")?;

    let mut config = Config::new(operator_key, args.content_dir)
        .with_addr(args.listen)
        .with_shutdown(shutdown_signal());

    if let Some(url) = args.content_base_url {
        config = config.with_content_base_url(url);
    }

    if let Some(path) = args.authorized_keys_file {
        let authorized_keys_data = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read authorized keys from {path:?}"))?;
        let authorized_key_policy: AuthorizedKeyPolicy = toml::from_str(&authorized_keys_data)
            .with_context(|| format!("failed to decode authorized keys from {path:?}"))?;
        config = config.with_record_policy(authorized_key_policy);
    }

    let metadata_extractor = metadata::MetadataExtractor::new();
    config = config.with_metadata_extractor(metadata_extractor);

    let config = match args.data_store {
        #[cfg(feature = "postgres")]
        DataStoreKind::Postgres => {
            use warg_server::datastore::PostgresDataStore;
            tracing::info!("using postgres data store");
            let database_url =
                get_opt_secret("database-url", args.database_url_file, args.database_url)?;
            let pg_store = PostgresDataStore::new(database_url)?;
            if args.database_run_migrations {
                tracing::info!("running any pending database migration(s)");
                pg_store.run_pending_migrations().await?;
            }
            config.with_data_store(pg_store)
        }
        DataStoreKind::Memory => {
            tracing::info!("using memory data store");
            config
        }
    };

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
