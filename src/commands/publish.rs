use super::CommonOptions;
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use std::{env, path::PathBuf};
use warg_client::PackageEntryInfo;
use warg_crypto::signing;
use warg_protocol::Version;

// TODO: convert this to proper CLI options.
fn demo_signing_key() -> Result<signing::PrivateKey> {
    let path = env::var("WARG_DEMO_USER_KEY")
        .context("WARG_DEMO_USER_KEY environment variable not set")?;

    path.parse().context("failed to parse signing key")
}

/// Publish a package to a warg registry.
#[derive(Subcommand)]
pub enum PublishCommand {
    /// Start a new package publish.
    Start(PublishStartCommand),
    /// Release a package version.
    Release(PublishReleaseCommand),
    /// List the pending publish operations.
    List(PublishListCommand),
    /// Abort a pending publish operation.
    Abort(PublishAbortCommand),
    /// Submit a pending publish operation.
    Submit(PublishSubmitCommand),
}

impl PublishCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        match self {
            Self::Start(cmd) => cmd.exec().await,
            Self::Release(cmd) => cmd.exec().await,
            Self::List(cmd) => cmd.exec().await,
            Self::Abort(cmd) => cmd.exec().await,
            Self::Submit(cmd) => cmd.exec().await,
        }
    }
}

/// Start a new package publish.
#[derive(Args)]
pub struct PublishStartCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// Use to initialize a new package.
    #[clap(long)]
    pub init: bool,
    /// The name of the package being published.
    #[clap(value_name = "NAME")]
    pub name: String,
}

impl PublishStartCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let mut client = self.common.create_client()?;
        let signing_key = demo_signing_key()?;

        if self.init {
            println!(
                "starting publish for new package `{name}`...",
                name = self.name
            );

            client
                .start_publish_init(self.name, signing_key.public_key())
                .await?;

            return Ok(());
        }

        println!("starting publish for package `{name}`...", name = self.name);
        client.start_publish(self.name).await?;
        Ok(())
    }
}

/// Release a package version.
#[derive(Args)]
pub struct PublishReleaseCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The version of the package being published.
    #[clap(long, short)]
    pub version: Version,
    /// The path to the package being published.
    #[clap(value_name = "PATH")]
    pub path: PathBuf,
}

impl PublishReleaseCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        println!(
            "queuing release of {version} with content `{path}`...",
            version = self.version,
            path = self.path.display()
        );

        let mut client = self.common.create_client()?;
        let content = tokio::fs::read(&self.path).await.with_context(|| {
            format!(
                "failed to read package content `{path}`",
                path = self.path.display()
            )
        })?;

        let mut storage_content = client.storage().create_content().await?;
        storage_content.write_all(content.as_slice()).await?;
        let digest = storage_content.finalize().await?;

        client.queue_release(self.version, digest).await?;
        Ok(())
    }
}

/// List the pending publish operations.
#[derive(Args)]
pub struct PublishListCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl PublishListCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let mut client = self.common.create_client()?;

        match client.storage().load_publish_info().await? {
            Some(info) => {
                println!(
                    "publishing package `{package}` ({entries} entries)\n",
                    package = info.package,
                    entries = info.entries.len()
                );

                if let Some(prev) = &info.prev {
                    println!("previous record hash: {prev}");
                } else {
                    println!("no previous record (this publish must init)");
                }

                for (i, entry) in info.entries.iter().enumerate() {
                    print!("{i} ");
                    match entry {
                        PackageEntryInfo::Init {
                            hash_algorithm,
                            key,
                        } => {
                            println!("init {hash_algorithm} - {key}")
                        }
                        PackageEntryInfo::Release { version, content } => {
                            println!("release {version} - {content}")
                        }
                    }
                }

                Ok(())
            }
            None => bail!("no pending publish operations"),
        }
    }
}

/// Abort a pending publish operation.
#[derive(Args)]
pub struct PublishAbortCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl PublishAbortCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        println!("aborting current publish...");

        let mut client = self.common.create_client()?;
        client.cancel_publish().await?;
        Ok(())
    }
}

/// Submit a pending publish operation.
#[derive(Args)]
pub struct PublishSubmitCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl PublishSubmitCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        println!("submitting current publish...");
        let signing_key = demo_signing_key()?;

        let mut client = self.common.create_client()?;
        client.submit_publish(&signing_key).await?;
        Ok(())
    }
}
