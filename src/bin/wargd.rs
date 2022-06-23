use std::net::SocketAddr;

use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use wasm_registry::server::Server;

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(long = "--addr", default_value = "127.0.0.1:9999")]
    addr: SocketAddr,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    Server::default().run(&args.addr).await;
}
