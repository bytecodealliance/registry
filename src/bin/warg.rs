use anyhow::Result;
use clap::Parser;
use std::process::exit;
use tracing_subscriber::EnvFilter;
use warg_cli::commands::{
    BundleCommand, ClearCommand, ConfigCommand, DependenciesCommand, DownloadCommand, InfoCommand,
    KeyCommand, LockCommand, LoginCommand, LogoutCommand, PublishCommand, ResetCommand,
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
    Lock(LockCommand),
    Bundle(BundleCommand),
    Dependencies(DependenciesCommand),
    Download(DownloadCommand),
    Update(UpdateCommand),
    #[clap(subcommand)]
    Publish(PublishCommand),
    Reset(ResetCommand),
    Clear(ClearCommand),
    Login(LoginCommand),
    Logout(LogoutCommand),
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
        WargCli::Lock(cmd) => cmd.exec().await,
        WargCli::Bundle(cmd) => cmd.exec().await,
        WargCli::Dependencies(cmd) => cmd.exec().await,
        WargCli::Download(cmd) => cmd.exec().await,
        WargCli::Update(cmd) => cmd.exec().await,
        WargCli::Publish(cmd) => cmd.exec().await,
        WargCli::Reset(cmd) => cmd.exec().await,
        WargCli::Clear(cmd) => cmd.exec().await,
        WargCli::Login(cmd) => cmd.exec().await,
        WargCli::Logout(cmd) => cmd.exec().await,
    } {
        if let Some(e) = e.downcast_ref::<ClientError>() {
            describe_client_error(e).await?;
        } else {
            eprintln!("error: {e:?}");
        }
        exit(1);
    }

    Ok(())
}

pub async fn describe_client_error(e: &ClientError) -> Result<()> {
    match e {
        ClientError::NoHomeRegistryUrl => {
            eprintln!("Registry not set. Use `config` or `login` subcommand to set registry.");
        }
        ClientError::PackageValidationFailed { name, inner } => {
            eprintln!("The log for package `{name}` validation failed: {inner}")
        }
        ClientError::PackageLogEmpty { name } => {
            eprintln!("The log for package `{name}` is empty (the registry could be lying)");
            eprintln!("see issue https://github.com/bytecodealliance/registry/issues/66");
        }
        ClientError::PackageDoesNotExist {
            name,
            has_auth_token,
        } => {
            eprintln!("Package `{name}` was not found or you do not have access.");
            if !has_auth_token {
                eprintln!("You may be required to login. Try: `warg login`");
            }
        }
        ClientError::PackageDoesNotExistWithHintHeader {
            name,
            has_auth_token,
            hint_namespace,
            hint_registry,
        } => {
            eprintln!(
                "Package `{name}` was not found or you do not have access.
The registry suggests using registry `{hint_registry}` for packages in namespace `{hint_namespace}`.");
            if !has_auth_token {
                eprintln!("You may be required to login. Try: `warg login`");
            }
        }
        ClientError::PackageVersionDoesNotExist { name, version } => {
            eprintln!("Package `{name}` version `{version}` was not found.")
        }
        ClientError::PackageVersionRequirementDoesNotExist { name, version } => {
            eprintln!(
                "Package `{name}` version that satisfies requirement `{version}` was not found."
            )
        }
        ClientError::MustInitializePackage {
            name,
            has_auth_token,
        } => {
            eprintln!("Package `{name}` is not initialized or you do not have access.");
            if !has_auth_token {
                eprintln!("You may be required to login. Try: `warg login`");
            }
            eprintln!("To initialize package: `warg publish init {name}`");
        }
        ClientError::CannotInitializePackage {
            name,
            init_record_id,
        } => {
            if init_record_id.is_some() {
                eprintln!(
                    "Package `{name}` was initialized but with a different record than you signed.
This may be expected behavior for registries that offer key management."
                )
            } else {
                eprintln!("Package `{name}` is already initialized.")
            }
        }
        ClientError::PublishRejected { name, reason, .. } => {
            eprintln!("Package `{name}` publish rejected: {reason}")
        }
        ClientError::ConflictPendingPublish {
            name,
            pending_record_id,
            ..
        } => {
            eprintln!("Package `{name}` publish rejected due to conflict with pending publish of record `{pending_record_id}`")
        }
        ClientError::Unauthorized(reason) => {
            eprintln!("Unauthorized: {reason}")
        }
        _ => {
            eprintln!("error: {e}")
        }
    }
    Ok(())
}
