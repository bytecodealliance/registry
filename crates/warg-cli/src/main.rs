mod data;
mod install;
mod publish;
mod registry_info;

use anyhow::Result;
use clap::{Parser, Subcommand};
use data::CliData;
use install::install;
use publish::{publish_command, PublishCommand};
use registry_info::RegistryInfo;
use warg_client::api;
use warg_crypto::signing;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    SetRegistry {
        registry: String,
    },
    Install {
        package: String,
    },
    Update,
    Publish {
        #[command(subcommand)]
        subcommand: PublishCommand,
    },
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let args = Args::parse();

    let demo_user_key = std::env::var("WARG_DEMO_USER_KEY")?;
    let demo_user_key: signing::PrivateKey = demo_user_key.parse()?;

    let data = CliData::new();

    match args.command {
        Commands::SetRegistry { registry } => set_registry(data, registry).await,
        Commands::Install { package } => install(data, package).await,
        Commands::Update => {
            todo!()
        }
        Commands::Publish { subcommand } => publish_command(data, demo_user_key, subcommand).await,
    }
}

async fn set_registry(data: CliData, url: String) -> Result<()> {
    let client = api::Client::new(url.clone());
    let checkpoint = client.latest_checkpoint().await?;

    let reg_info = RegistryInfo::new(url, checkpoint);
    data.set_registry_info(&reg_info)?;
    Ok(())
}

fn advise_set_registry() {
    println!("Warg must have a registry set.");
    println!("Use 'set-registry' to select a registry.")
}

fn advise_end_publish() {
    eprintln!("Warg must not be publishing already.");
    eprintln!("Use 'publish submit' or 'publish abort' to resolve this publish.");
}

fn advise_start_publish() {
    eprintln!("Warg must be in publishing mode.");
    eprintln!("Use 'create-package' or 'publish start' to begin publishing.");
}
