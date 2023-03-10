use super::CommonOptions;
use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use clap::{Args, Subcommand};
use futures::{Stream, TryStreamExt};
use std::{env, path::PathBuf, pin::Pin};
use tokio::io::BufReader;
use tokio_util::io::ReaderStream;
use warg_client::{
    storage::{
        ClientStorage, FileSystemStorage, PackageInfo, PublishEntry, PublishInfo, RegistryInfo,
    },
    Client,
};
use warg_crypto::{hash::DynHash, signing};
use warg_protocol::Version;

// TODO: convert this to proper CLI options.
fn demo_signing_key() -> Result<signing::PrivateKey> {
    env::var("WARG_DEMO_USER_KEY")
        .context("WARG_DEMO_USER_KEY environment variable not set")?
        .parse()
        .context("failed to parse signing key")
}

/// Publish a package to a warg registry.
#[derive(Subcommand)]
pub enum PublishCommand {
    /// Release a package version.
    Release(PublishReleaseCommand),
    /// List the pending publish operations.
    List(PublishListCommand),
    /// Abort a pending publish operation
    Abort(PublishAbortCommand),
    /// Submit a pending publish operation.
    Submit(PublishSubmitCommand),
}

impl PublishCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        match self {
            Self::Release(cmd) => cmd.exec().await,
            Self::List(cmd) => cmd.exec().await,
            Self::Abort(cmd) => cmd.exec().await,
            Self::Submit(cmd) => cmd.exec().await,
        }
    }
}

/// Publish a package to a warg registry.
#[derive(Args)]
#[clap(disable_version_flag = true)]
pub struct PublishReleaseCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// Use to initialize a new package.
    #[clap(long)]
    pub init: bool,
    /// Queue the release for publishing with the `publish submit` command.
    #[clap(long)]
    pub queue: bool,
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
        let storage = self.common.lock_storage()?;

        let mut info = storage
            .load_publish_info()
            .await?
            .unwrap_or_else(|| PublishInfo {
                package: self.name.clone(),
                init: self.init,
                entries: Default::default(),
            });

        if info.package != self.name {
            bail!(
                "there is already a pending publish operation for package `{name}`",
                name = info.package
            );
        }

        if self.queue {
            if self.init && !info.entries.is_empty() {
                bail!("only the first pending publish operation may initialize the package");
            }
        } else if self.init != info.init || !info.entries.is_empty() {
            bail!("there is already a pending publish operation; use the `--queue` option to add the release to the queue");
        }

        let content = storage
            .store_content(
                Box::pin(
                    ReaderStream::new(BufReader::new(
                        tokio::fs::File::open(&self.path).await.with_context(|| {
                            format!("failed to open `{path}`", path = self.path.display())
                        })?,
                    ))
                    .map_err(|e| anyhow!(e)),
                ),
                None,
            )
            .await?;

        info.entries.push(PublishEntry::Release {
            version: self.version.clone(),
            content,
        });

        if self.queue {
            storage.store_publish_info(Some(&info)).await?;

            println!(
                "queued release of version {version} for package `{package}`",
                version = self.version,
                package = self.name
            );

            return Ok(());
        }

        let signing_key = demo_signing_key()?;
        let mut client = Client::new(ImmediatePublishStorage {
            storage: &storage,
            info: &info,
        })
        .await?;

        client.publish(&signing_key).await?;

        println!(
            "published version {version} of package `{name}`",
            version = self.version,
            name = self.name
        );
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
        let storage = self.common.lock_storage()?;
        match storage.load_publish_info().await? {
            Some(info) => {
                println!(
                    "publishing {new}package `{package}` with {entries} record(s) to publish\n",
                    new = if info.init { "new " } else { "" },
                    package = info.package,
                    entries = info.entries.len()
                );

                for (i, entry) in info.entries.iter().enumerate() {
                    print!("record {i}: ");
                    match entry {
                        PublishEntry::Release { version, content } => {
                            println!("release {version} with content digest `{content}`")
                        }
                    }
                }
            }
            None => println!("no pending publish operation to list"),
        }

        Ok(())
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
        let storage = self.common.lock_storage()?;
        if storage.has_publish_info() {
            storage.store_publish_info(None).await?;
            println!("aborted the pending publish operation");
        } else {
            println!("no pending publish operation to abort");
        }

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
        let mut client = self.common.create_client().await?;
        if client.storage().has_publish_info() {
            println!("submitting pending publish operation...");
            client.publish(&demo_signing_key()?).await?;
        } else {
            println!("no pending publish operation to submit");
        }

        Ok(())
    }
}

// A client storage wrapper that doesn't hit disk to provide publishing information.
struct ImmediatePublishStorage<'a> {
    storage: &'a FileSystemStorage,
    info: &'a PublishInfo,
}

#[async_trait]
impl<'a> ClientStorage for ImmediatePublishStorage<'a> {
    async fn load_registry_info(&self) -> Result<Option<RegistryInfo>> {
        self.storage.load_registry_info().await
    }

    async fn store_registry_info(&self, info: &RegistryInfo) -> Result<()> {
        self.storage.store_registry_info(info).await
    }

    async fn load_packages(&self) -> Result<Vec<PackageInfo>> {
        self.storage.load_packages().await
    }

    async fn load_package_info(&self, package: &str) -> Result<Option<PackageInfo>> {
        self.storage.load_package_info(package).await
    }

    async fn store_package_info(&self, info: &PackageInfo) -> Result<()> {
        self.storage.store_package_info(info).await
    }

    fn has_publish_info(&self) -> bool {
        true
    }

    async fn load_publish_info(&self) -> Result<Option<PublishInfo>> {
        Ok(Some(self.info.clone()))
    }

    async fn store_publish_info(&self, _info: Option<&PublishInfo>) -> Result<()> {
        Ok(())
    }

    fn content_path(&self, digest: &DynHash) -> Option<PathBuf> {
        ClientStorage::content_path(self.storage, digest)
    }

    async fn load_content(
        &self,
        digest: &DynHash,
    ) -> Result<Option<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send + Sync>>>> {
        self.storage.load_content(digest).await
    }

    async fn store_content(
        &self,
        stream: Pin<Box<dyn Stream<Item = Result<Bytes>> + Send + Sync>>,
        expected_digest: Option<&DynHash>,
    ) -> Result<DynHash> {
        self.storage.store_content(stream, expected_digest).await
    }
}
