use super::CommonOptions;
use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use clap::{Args, Subcommand};
use futures::{Stream, TryStreamExt};
use std::{env, future::Future, path::PathBuf, pin::Pin};
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

/// Used to enqueue a publish entry if there is a pending publish.
/// Returns `Ok(None)` if the entry was enqueued or `Ok(Some(entry))` if there
/// was no pending publish.
async fn enqueue<'a, T>(
    storage: &'a FileSystemStorage,
    name: &str,
    entry: impl FnOnce(&'a FileSystemStorage) -> T,
) -> Result<Option<PublishEntry>>
where
    T: Future<Output = Result<PublishEntry>> + 'a,
{
    match storage.load_publish_info().await? {
        Some(mut info) => {
            if info.package != name {
                bail!(
                    "there is already publish in progress for package `{package}`",
                    package = info.package
                );
            }

            let entry = entry(storage).await?;

            if matches!(entry, PublishEntry::Init) && info.initializing() {
                bail!(
                    "there is already a pending initializing for package `{package}`",
                    package = name
                );
            }

            info.entries.push(entry);
            storage.store_publish_info(Some(&info)).await?;
            Ok(None)
        }
        None => Ok(Some(entry(storage).await?)),
    }
}

/// Submits a publish to the registry.
async fn submit(storage: &FileSystemStorage, info: &PublishInfo) -> Result<()> {
    let signing_key = demo_signing_key()?;
    let mut client = Client::new(InMemoryPublishStorage { storage, info }).await?;
    client.publish(&signing_key).await?;
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
        let storage = self.common.lock_storage()?;

        match enqueue(&storage, &self.name, |_| {
            std::future::ready(Ok(PublishEntry::Init))
        })
        .await?
        {
            Some(entry) => {
                submit(
                    &storage,
                    &PublishInfo {
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
        let storage = self.common.lock_storage()?;
        let path = self.path.clone();
        let version = self.version.clone();
        match enqueue(&storage, &self.name, move |s| async move {
            let content = s
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
                    &storage,
                    &PublishInfo {
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
        let storage = self.common.lock_storage()?;
        match storage.load_publish_info().await? {
            Some(info) => bail!("a publish is already in progress for package `{package}`; use `publish abort` to abort the current publish", package = info.package),
            None => {
                storage.store_publish_info(Some(&PublishInfo {
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
        let storage = self.common.lock_storage()?;
        match storage.load_publish_info().await? {
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
        let storage = self.common.lock_storage()?;
        if storage.has_publish_info() {
            storage.store_publish_info(None).await?;
            println!("aborted the pending publish");
        } else {
            bail!("no pending publish to abort");
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
        let storage = self.common.lock_storage()?;
        match storage.load_publish_info().await? {
            Some(info) => {
                println!(
                    "submitting publish for package `{package}`...",
                    package = info.package
                );

                submit(&storage, &info).await?;

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

// A client storage wrapper that doesn't hit disk to provide publishing information.
struct InMemoryPublishStorage<'a> {
    storage: &'a FileSystemStorage,
    info: &'a PublishInfo,
}

#[async_trait]
impl<'a> ClientStorage for InMemoryPublishStorage<'a> {
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

    async fn store_publish_info(&self, info: Option<&PublishInfo>) -> Result<()> {
        assert!(info.is_none());
        self.storage.store_publish_info(info).await
    }

    fn content_location(&self, digest: &DynHash) -> Option<PathBuf> {
        self.storage.content_location(digest)
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
