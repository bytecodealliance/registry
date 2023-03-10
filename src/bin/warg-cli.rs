use anyhow::Result;
use clap::Parser;
use std::process::exit;
use warg_cli::commands::{
    InfoCommand, InitCommand, InstallCommand, PublishCommand, RunCommand, UpdateCommand,
};
use warg_client::ClientError;

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
    Info(InfoCommand),
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
        WargCli::Info(cmd) => cmd.exec().await,
        WargCli::Init(cmd) => cmd.exec().await,
        WargCli::Install(cmd) => cmd.exec().await,
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
        ClientError::RegistryNotSet => {
            eprintln!("error: registry was not initialized; use the `init` command to get started");
        }
        ClientError::AlreadyPublishing => {
            eprintln!("error: a publish operation is already in progress");
            eprintln!(
                "use `publish submit` or `publish abort` to resolve the current publish operation"
            );
        }
        ClientError::NotPublishing => {
            eprintln!("error: there is no pending publish operation; use `publish start` to begin publishing");
        }
        ClientError::InitAlreadyExistingPackage => {
            eprintln!("error: selected package name already exists; choose a different name for creating a new package");
        }
        ClientError::PublishToNonExistingPackage => {
            eprintln!("error: selected package does not exist; create the package with `publish start --init`");
        }
        ClientError::NeededContentNotFound { digest } => {
            eprintln!("error: content needed by the current publish operation was not found");
            eprintln!("content with digest `{digest}` not present in contents directory");
            eprintln!("this indicates that the directory has been modified or that a previous");
            eprintln!("`publish release` command was interrupted.");
        }
        ClientError::RequestedPackageOmitted { package } => {
            eprintln!(
                "note: the registry did not provide the requested data about package `{package}`"
            );
        }
        ClientError::PackageValidationError { package, inner } => {
            eprintln!("error: the log for package `{package}` is invalid / corrupt: {inner:?}");
        }
        ClientError::PackageLogEmpty => {
            eprintln!("error: the package could be empty or the registry could be lying.");
            eprintln!("see issue https://github.com/bytecodealliance/registry/issues/66");
        }
        ClientError::OtherError(e) => {
            eprintln!("error: {:?}", e);
        }
    }
}
