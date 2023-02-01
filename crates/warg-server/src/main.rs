use std::{net::SocketAddr, path::PathBuf};

use anyhow::Result;
use clap::Parser;

use tracing_subscriber::filter::LevelFilter;
use warg_server::Config;

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
    tracing::debug!("Args: {args:?}");

    let demo_operator_key = std::env::var("WARG_DEMO_OPERATOR_KEY")?;
    let signing_key = demo_operator_key.parse()?;

    let base_url = format!("http://{}", args.listen.to_string());
    let mut config = Config::new(base_url, signing_key);
    if let Some(path) = args.content_dir {
        config.enable_content_service(path);
    }
    tracing::debug!("Config: {config:?}");

    let router = config.build_router()?;

    tracing::info!("Listening on {:?}", args.listen);
    axum::Server::try_bind(&args.listen)?
        .serve(router.into_make_service())
        .await?;

    Ok(())
}
