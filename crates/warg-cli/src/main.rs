mod publish;

// FIXME: delete
mod demo;

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use publish::publish_command;
use warg_client::{Client, ClientError, FileSystemStorage, RegistryInfo};
use warg_crypto::signing;
use warg_protocol::Version;

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
    Run {
        name: String,
        args: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum PublishCommand {
    Start {
        #[clap(long)]
        name: String,
        #[clap(long)]
        init: bool,
    },
    Release {
        version: Version,
        #[clap(long)]
        path: PathBuf,
    },
    List,
    Abort,
    Submit,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let args = Args::parse();

    describe_command(&args.command);

    match run_command(args.command).await {
        Ok(()) => {
            println!("Done");
        }
        Err(error) => describe_error(&error),
    }
    Ok(())
}

async fn run_command(command: Commands) -> Result<(), ClientError> {
    let storage = FileSystemStorage::new();
    let mut client = Client::new(Box::new(storage));

    match command {
        Commands::SetRegistry { registry } => set_registry(client, registry).await,
        Commands::Install { package } => client.install(package).await,
        Commands::Update => client.update().await,
        Commands::Publish { subcommand } => {
            let demo_user_key =
                std::env::var("WARG_DEMO_USER_KEY").with_context(|| "User key not found")?;
            let demo_user_key: signing::PrivateKey =
                demo_user_key.parse().with_context(|| "User key invalid")?;

            publish_command(client, demo_user_key, subcommand).await
        }
        Commands::Run { name, args } => {
            let content_digest = client.get_latest_version(&name).await?;
            let storage = FileSystemStorage::new();
            let path = storage.content_path(&content_digest);
            demo::run_wasm(path, &args)
        }
    }
}

async fn set_registry(mut client: Client, url: String) -> Result<(), ClientError> {
    let reg_info = client.storage().load_registry_info().await?;
    if let Some(_) = reg_info {
        todo!("Switching between registries not supported. Reset client to continue.");
    } else {
        client
            .storage()
            .store_registry_info(&RegistryInfo {
                url,
                checkpoint: None,
            })
            .await?;
        Ok(())
    }
}

fn describe_command(command: &Commands) {
    match command {
        Commands::SetRegistry { registry } => {
            println!("Setting the registry URL to {}...", registry);
        }
        Commands::Install { package } => {
            println!("Installing package {}...", package);
        }
        Commands::Update => {
            println!("Updating installed packages to registry latest...");
        }
        Commands::Publish { subcommand } => match subcommand {
            PublishCommand::Start { name, init } => {
                if *init {
                    println!("Starting publish for new package \"{}\"...", name);
                } else {
                    println!("Starting publish for package \"{}\"...", name);
                }
            }
            PublishCommand::Release { version, path } => {
                println!("Queuing release of {} with content {:?}...", version, path);
            }
            PublishCommand::List => {
                println!("Listing publish status...");
            }
            PublishCommand::Abort => {
                println!("Aborting current publish...");
            }
            PublishCommand::Submit => {
                println!("Submitting current publish...");
            }
        },
        Commands::Run { name, args: _ } => {
            println!("Running package {}", name);
        }
    }
}

/// Prints error messages specialized to the CLI environment.
fn describe_error(error: &ClientError) {
    match error {
        ClientError::RegistryNotSet => {
            eprintln!("No registry selected.");
            eprintln!("Use 'set-registry' to select a registry.");
        }
        ClientError::AlreadyPublishing => {
            eprintln!("Already publishing.");
            eprintln!("Use 'publish submit' or 'publish abort' to resolve this publish.");
        }
        ClientError::NotPublishing => {
            eprintln!("Not currently publishing.");
            eprintln!("Use 'publish start' to begin publishing.");
        }
        ClientError::InitAlreadyExistingPackage => {
            eprintln!("Selected package name already exists.");
            eprintln!("Choose a new package name to create.")
        }
        ClientError::PublishToNonExistingPackage => {
            eprintln!("Selected package doesn't exist.");
            eprintln!("Create this package with 'publish start ... --init'.");
        }
        ClientError::NeededContentNotFound { digest } => {
            eprintln!("Content needed in the current publish was not found.");
            eprintln!(
                "Content with digest {} not present in contents directory.",
                digest
            );
            eprintln!("This indicates that the directory has been modified or that a previous");
            eprintln!("'publish release...' command was interrupted.");
        }
        ClientError::RequestedPackageOmitted { package } => {
            eprintln!(
                "Alert: The registry did not provide the requested data about package {}",
                package
            );
        }
        ClientError::PackageValidationError { package, inner } => {
            eprintln!("Alert: Package log for {} is invalid / corrupt.", package);
            eprintln!("{:?}", inner);
        }
        ClientError::PackageLogEmpty => {
            eprintln!("This package could be empty or the registry could be lying.");
            eprintln!("See issue https://github.com/bytecodealliance/registry/issues/66");
        }
        ClientError::OtherError(error) => {
            eprintln!("Error encountered while processing command.");
            eprintln!("{:?}", error);
        }
    }
}
