use anyhow::Result;
use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Confirm};
use std::process::exit;
use tracing_subscriber::EnvFilter;
use warg_cli::commands::{
    BundleCommand, ClearCommand, ConfigCommand, DependenciesCommand, DownloadCommand, InfoCommand,
    KeyCommand, LockCommand, LoginCommand, LogoutCommand, PublishCommand, ResetCommand, Retry,
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
        WargCli::Lock(cmd) => cmd.exec(None).await,
        WargCli::Bundle(cmd) => cmd.exec(None).await,
        WargCli::Dependencies(cmd) => cmd.exec(None).await,
        WargCli::Download(cmd) => cmd.exec(None).await,
        WargCli::Update(cmd) => cmd.exec(None).await,
        WargCli::Publish(cmd) => cmd.exec(None).await,
        WargCli::Reset(cmd) => cmd.exec().await,
        WargCli::Clear(cmd) => cmd.exec().await,
        WargCli::Login(cmd) => cmd.exec().await,
        WargCli::Logout(cmd) => cmd.exec().await,
    } {
        if let Some(e) = e.downcast_ref::<ClientError>() {
            describe_client_error_or_retry(e).await?;
        } else {
            eprintln!("error: {e:?}");
        }
        exit(1);
    }

    Ok(())
}

async fn describe_client_error_or_retry(e: &ClientError) -> Result<()> {
    match e {
        ClientError::NoHomeRegistryUrl => {
            eprintln!("error: {e}; use the `config` subcommand to set a home registry URL");
        }
        ClientError::PackageValidationFailed { name, inner } => {
            eprintln!("error: the log for package `{name}` is invalid: {inner}")
        }
        ClientError::PackageLogEmpty { name } => {
            eprintln!("error: the log for package `{name}` is empty (the registry could be lying)");
            eprintln!("see issue https://github.com/bytecodealliance/registry/issues/66");
        }
        ClientError::PackageDoesNotExistWithHint { name, hint } => {
            let hint_reg = hint.to_str().unwrap();
            let mut terms = hint_reg.split('=');
            let namespace = terms.next();
            let registry = terms.next();
            if let (Some(namespace), Some(registry)) = (namespace, registry) {
                let prompt = format!(
                "The package `{}`, does not exist in the registry you're using.\nHowever, the package namespace `{namespace}` does exist in the registry at {registry}.\nWould you like to configure your warg cli to use this registry for packages with this namespace in the future? y/N\n",
                name.name()
              );
                if Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt(prompt)
                    .interact()
                    .unwrap()
                {
                    if let Err(e) = match WargCli::parse() {
                        WargCli::Config(cmd) => cmd.exec().await,
                        WargCli::Info(cmd) => cmd.exec().await,
                        WargCli::Key(cmd) => cmd.exec().await,
                        WargCli::Lock(cmd) => {
                            cmd.exec(Some(Retry::new(
                                namespace.to_string(),
                                registry.to_string(),
                            )))
                            .await
                        }
                        WargCli::Bundle(cmd) => {
                            cmd.exec(Some(Retry::new(
                                namespace.to_string(),
                                registry.to_string(),
                            )))
                            .await
                        }
                        WargCli::Dependencies(cmd) => {
                            cmd.exec(Some(Retry::new(
                                namespace.to_string(),
                                registry.to_string(),
                            )))
                            .await
                        }
                        WargCli::Download(cmd) => {
                            cmd.exec(Some(Retry::new(
                                namespace.to_string(),
                                registry.to_string(),
                            )))
                            .await
                        }
                        WargCli::Update(cmd) => {
                            cmd.exec(Some(Retry::new(
                                namespace.to_string(),
                                registry.to_string(),
                            )))
                            .await
                        }
                        WargCli::Publish(cmd) => {
                            cmd.exec(Some(Retry::new(
                                namespace.to_string(),
                                registry.to_string(),
                            )))
                            .await
                        }
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
                }
            }
        }
        _ => {
            eprintln!("error: {e}")
        }
    }
    Ok(())
}

async fn describe_client_error(e: &ClientError) -> Result<()> {
    match e {
        ClientError::NoHomeRegistryUrl => {
            eprintln!("error: {e}; use the `config` subcommand to set a default URL");
        }
        ClientError::PackageValidationFailed { name, inner } => {
            eprintln!("error: the log for package `{name}` is invalid: {inner}")
        }
        ClientError::PackageLogEmpty { name } => {
            eprintln!("error: the log for package `{name}` is empty (the registry could be lying)");
            eprintln!("see issue https://github.com/bytecodealliance/registry/issues/66");
        }
        _ => {
            eprintln!("error: {e}")
        }
    }
    Ok(())
}
