use anyhow::Result;
use clap::Parser;
use std::process::exit;
use tracing_subscriber::EnvFilter;
use warg_cli::commands::{
    ConfigCommand, DownloadCommand, InfoCommand, KeyCommand, PublishCommand, RunCommand,
    UpdateCommand,
};
use warg_client::ClientError;

fn version() -> &'static str {
    option_env!("CARGO_VERSION_INFO").unwrap_or(env!("CARGO_PKG_VERSION"))
}

/// Warg component registry client.
#[derive(Parser)]
#[clap(
    bin_name = "warg",
    version,
    propagate_version = true,
    arg_required_else_help = true
)]
#[command(version = version())]
enum WargCli {
    Config(ConfigCommand),
    Info(InfoCommand),
    Key(KeyCommand),
    Download(DownloadCommand),
    Update(UpdateCommand),
    #[clap(subcommand)]
    Publish(PublishCommand),
    Run(RunCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    if let Err(e) = match WargCli::parse() {
        WargCli::Config(cmd) => cmd.exec().await,
        WargCli::Info(cmd) => cmd.exec().await,
        WargCli::Key(cmd) => cmd.exec().await,
        WargCli::Download(cmd) => cmd.exec().await,
        WargCli::Update(cmd) => cmd.exec().await,
        WargCli::Publish(cmd) => cmd.exec().await,
        WargCli::Run(cmd) => cmd.exec().await,
    } {
        if let Some(e) = e.downcast_ref::<ClientError>() {
            describe_client_error(e);
        } else {
            eprintln!("error: {e:?}");
        }
        exit(1);
    }

    Ok(())
}

fn describe_client_error(e: &ClientError) {
    match e {
        ClientError::NoDefaultUrl => {
            eprintln!("error: {e}; use the `config` subcommand to set a default URL");
        }
        ClientError::PackageValidationFailed { id, inner } => {
            eprintln!("error: the log for package `{id}` is invalid: {inner}")
        }
        ClientError::PackageLogEmpty { id } => {
            eprintln!("error: the log for package `{id}` is empty (the registry could be lying)");
            eprintln!("see issue https://github.com/bytecodealliance/registry/issues/66");
        }
        _ => eprintln!("error: {e}"),
    }
}
