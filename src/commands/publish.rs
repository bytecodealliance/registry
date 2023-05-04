use super::CommonOptions;
use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Subcommand};
use futures::TryStreamExt;
use std::{env, future::Future, path::PathBuf};
use tokio::io::BufReader;
use tokio_util::io::ReaderStream;
use warg_client::{
    storage::{ContentStorage as _, RegistryStorage as _, PublishEntry, PublishInfo},
    FileSystemClient,
};
use warg_crypto::signing;
use warg_protocol::Version;

// TODO: convert this to proper CLI options.
fn demo_signing_key() -> Result<signing::PrivateKey> {
    env::var("WARG_DEMO_USER_KEY")
        .context("WARG_DEMO_USER_KEY environment variable not set")?
        .parse()
        .context("failed to parse signing key")
}

/// Used to enqueue a publish entry if there is a pending publish.
/// Returns `Ok(None)` if the entry was enqueued or `Ok(Some(entry))` if there
/// was no pending publish.
async fn enqueue<'a, T>(
    client: &'a FileSystemClient,
    name: &str,
    entry: impl FnOnce(&'a FileSystemClient) -> T,
) -> Result<Option<PublishEntry>>
where
    T: Future<Output = Result<PublishEntry>> + 'a,
{
    match client.packages().load_publish("dogfood").await? {
        Some(mut info) => {
            if info.package != name {
                bail!(
                    "there is already publish in progress for package `{package}`",
                    package = info.package
                );
            }

            let entry = entry(client).await?;

            if matches!(entry, PublishEntry::Init) && info.initializing() {
                bail!(
                    "there is already a pending initializing for package `{package}`",
                    package = name
                );
            }

            info.entries.push(entry);
            client.packages().store_publish("dogfood", Some(&info)).await?;
            Ok(None)
        }
        None => Ok(Some(entry(client).await?)),
    }
}

/// Submits a publish to the registry.
async fn submit(client: &FileSystemClient, info: PublishInfo) -> Result<()> {
    let signing_key = demo_signing_key()?;
    client.publish_with_info(&signing_key, info).await?;
    Ok(())
}

/// Publish a package to a warg registry.
#[derive(Subcommand)]
pub enum PublishCommand {
    /// Initialize a new package.
    Init(PublishInitCommand),
    /// Release a package version.
    Release(PublishReleaseCommand),
    /// Start a new pending publish.
    Start(PublishStartCommand),
    /// List the records in a pending publish.
    List(PublishListCommand),
    /// Abort a pending publish.
    Abort(PublishAbortCommand),
    /// Submit a pending publish.
    Submit(PublishSubmitCommand),
}

impl PublishCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        match self {
            Self::Init(cmd) => cmd.exec().await,
            Self::Release(cmd) => cmd.exec().await,
            Self::Start(cmd) => cmd.exec().await,
            Self::List(cmd) => cmd.exec().await,
            Self::Abort(cmd) => cmd.exec().await,
            Self::Submit(cmd) => cmd.exec().await,
        }
    }
}

/// Initialize a new package.
#[derive(Args)]
#[clap(disable_version_flag = true)]
pub struct PublishInitCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The name of the package being initialized.
    #[clap(value_name = "NAME")]
    pub name: String,
}

impl PublishInitCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        match enqueue(&client, &self.name, |_| {
            std::future::ready(Ok(PublishEntry::Init))
        })
        .await?
        {
            Some(entry) => {
                submit(
                    &client,
                    PublishInfo {
                        package: self.name.clone(),
                        entries: vec![entry],
                    },
                )
                .await?;

                println!(
                    "published initialization of package `{name}`",
                    name = self.name
                );
            }
            None => {
                println!(
                    "added initialization of package `{package}` to pending publish",
                    package = self.name
                );
            }
        }

        Ok(())
    }
}

/// Publish a package to a warg registry.
#[derive(Args)]
#[clap(disable_version_flag = true)]
pub struct PublishReleaseCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The name of the package being published.
    #[clap(long, short, value_name = "NAME")]
    pub name: String,
    /// The version of the package being published.
    #[clap(long, short, value_name = "VERSION")]
    pub version: Version,
    /// The path to the package being published.
    #[clap(value_name = "PATH")]
    pub path: PathBuf,
}

impl PublishReleaseCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        let path = self.path.clone();
        let version = self.version.clone();
        match enqueue(&client, &self.name, move |c| async move {
            let content = c
                .content()
                .store_content(
                    Box::pin(
                        ReaderStream::new(BufReader::new(
                            tokio::fs::File::open(&path).await.with_context(|| {
                                format!("failed to open `{path}`", path = path.display())
                            })?,
                        ))
                        .map_err(|e| anyhow!(e)),
                    ),
                    None,
                )
                .await?;

            Ok(PublishEntry::Release { version, content })
        })
        .await?
        {
            Some(entry) => {
                submit(
                    &client,
                    PublishInfo {
                        package: self.name.clone(),
                        entries: vec![entry],
                    },
                )
                .await?;

                println!(
                    "published version {version} of package `{name}`",
                    version = self.version,
                    name = self.name
                );
            }
            None => {
                println!(
                    "added release of version {version} for package `{package}` to pending publish",
                    version = self.version,
                    package = self.name
                );
            }
        }

        Ok(())
    }
}

/// Start a new pending publish.
#[derive(Args)]
#[clap(disable_version_flag = true)]
pub struct PublishStartCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The name of the package being published.
    #[clap(value_name = "NAME")]
    pub name: String,
}

impl PublishStartCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        match client.packages().load_publish("dogfood").await? {
            Some(info) => bail!("a publish is already in progress for package `{package}`; use `publish abort` to abort the current publish", package = info.package),
            None => {
                client.packages().store_publish("dogfood", Some(&PublishInfo {
                    package: self.name.clone(),
                    entries: Default::default(),
                }))
                .await?;

                println!(
                    "started new pending publish for package `{name}`",
                    name = self.name
                );
                Ok(())
            },
        }
    }
}

/// List the records in a pending publish.
#[derive(Args)]
pub struct PublishListCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl PublishListCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        match client.packages().load_publish("dogfood").await? {
            Some(info) => {
                println!(
                    "publishing package `{package}` with {count} record(s) to publish\n",
                    package = info.package,
                    count = info.entries.len()
                );

                for (i, entry) in info.entries.iter().enumerate() {
                    print!("record {i}: ");
                    match entry {
                        PublishEntry::Init => {
                            println!("initialize package");
                        }
                        PublishEntry::Release { version, content } => {
                            println!("release {version} with content digest `{content}`")
                        }
                    }
                }
            }
            None => bail!("no pending publish to list"),
        }

        Ok(())
    }
}

/// Abort a pending publish.
#[derive(Args)]
pub struct PublishAbortCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl PublishAbortCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        match client.packages().load_publish("dogfood").await? {
            Some(info) => {
                client.packages().store_publish("dogfood", None).await?;
                println!(
                    "aborted the pending publish for package `{package}`",
                    package = info.package
                );
            }
            None => bail!("no pending publish to abort"),
        }

        Ok(())
    }
}

/// Submit a pending publish.
#[derive(Args)]
pub struct PublishSubmitCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl PublishSubmitCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        match client.packages().load_publish("dogfood").await? {
            Some(info) => {
                println!(
                    "submitting publish for package `{package}`...",
                    package = info.package
                );

                submit(&client, info.clone()).await?;

                client.packages().store_publish("dogfood", None).await?;

                for entry in &info.entries {
                    match entry {
                        PublishEntry::Init => {
                            println!(
                                "published initialization of package `{package}`",
                                package = info.package
                            );
                        }
                        PublishEntry::Release { version, .. } => {
                            println!(
                                "published version {version} of package `{package}`",
                                version = version,
                                package = info.package,
                            );
                        }
                    }
                }
            }
            None => bail!("no pending publish to submit"),
        }

        Ok(())
    }
}
