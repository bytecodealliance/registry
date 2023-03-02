use anyhow::Result;
use clap::Parser;
use std::process::exit;
use warg_cli::commands::{InitCommand, InstallCommand, PublishCommand, RunCommand, UpdateCommand};

fn version() -> &'static str {
    option_env!("CARGO_VERSION_INFO").unwrap_or(env!("CARGO_PKG_VERSION"))
}

/// Warg component registry client.
#[derive(Parser)]
#[clap(
    bin_name = "warg-cli",
    version,
    propagate_version = true,
    arg_required_else_help = true
)]
#[command(version = version())]
enum WargCli {
    Init(InitCommand),
    Install(InstallCommand),
    Update(UpdateCommand),
    #[clap(subcommand)]
    Publish(PublishCommand),
    Run(RunCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = match WargCli::parse() {
        WargCli::Init(cmd) => cmd.exec().await,
        WargCli::Install(cmd) => cmd.exec().await,
        WargCli::Update(cmd) => cmd.exec().await,
        WargCli::Publish(cmd) => cmd.exec().await,
        WargCli::Run(cmd) => cmd.exec().await,
    } {
        eprintln!("error: {e:?}");
        exit(1);
    }

    Ok(())
}
