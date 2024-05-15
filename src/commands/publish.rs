use super::CommonOptions;
use anyhow::{anyhow, bail, Context, Result};
use clap::{Args, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm};
use futures::TryStreamExt;
use itertools::Itertools;
use std::{future::Future, path::PathBuf, time::Duration};
use tokio::io::BufReader;
use tokio_util::io::ReaderStream;
use warg_client::{
    storage::{ContentStorage as _, PublishEntry, PublishInfo, RegistryStorage as _},
    FileSystemClient,
};
use warg_crypto::{
    hash::AnyHash,
    signing::{KeyID, PublicKey},
};
use warg_protocol::{
    package::Permission,
    registry::{PackageName, RecordId},
    Version,
};

const DEFAULT_WAIT_INTERVAL: Duration = Duration::from_secs(1);

/// Used to enqueue a publish entry if there is a pending publish.
/// Returns `Ok(None)` if the entry was enqueued or `Ok(Some(entry))` if there
/// was no pending publish.
async fn enqueue<'a, T>(
    client: &'a FileSystemClient,
    name: &PackageName,
    entry: impl FnOnce(&'a FileSystemClient) -> T,
) -> Result<Option<PublishEntry>>
where
    T: Future<Output = Result<PublishEntry>> + 'a,
{
    match client.registry().load_publish().await? {
        Some(mut info) => {
            if &info.name != name {
                bail!(
                    "there is already publish in progress for package `{name}`",
                    name = info.name
                );
            }

            let entry = entry(client).await?;

            if matches!(entry, PublishEntry::Init) && info.initializing() {
                bail!("there is already a pending initializing for package `{name}`");
            }

            info.entries.push(entry);
            client.registry().store_publish(Some(&info)).await?;
            Ok(None)
        }
        None => Ok(Some(entry(client).await?)),
    }
}

/// Publish a package to a warg registry.
#[derive(Subcommand)]
pub enum PublishCommand {
    /// Initialize a new package.
    Init(PublishInitCommand),
    /// Release a package version.
    Release(PublishReleaseCommand),
    /// Yank a package version.
    Yank(PublishYankCommand),
    /// Grant permissions for the package.
    Grant(PublishGrantCommand),
    /// Revoke permissions for the package.
    Revoke(PublishRevokeCommand),
    /// Start a new pending publish.
    Start(PublishStartCommand),
    /// List the records in a pending publish.
    List(PublishListCommand),
    /// Abort a pending publish.
    Abort(PublishAbortCommand),
    /// Submit a pending publish.
    Submit(PublishSubmitCommand),
    /// Wait for a pending publish to complete.
    Wait(PublishWaitCommand),
}

impl PublishCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        match self {
            Self::Init(cmd) => cmd.exec().await,
            Self::Release(cmd) => cmd.exec().await,
            Self::Yank(cmd) => cmd.exec().await,
            Self::Grant(cmd) => cmd.exec().await,
            Self::Revoke(cmd) => cmd.exec().await,
            Self::Start(cmd) => cmd.exec().await,
            Self::List(cmd) => cmd.exec().await,
            Self::Abort(cmd) => cmd.exec().await,
            Self::Submit(cmd) => cmd.exec().await,
            Self::Wait(cmd) => cmd.exec().await,
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
    /// The package name being initialized.
    #[clap(value_name = "PACKAGE")]
    pub name: PackageName,
    /// Whether to wait for the publish to complete.
    #[clap(long)]
    pub no_wait: bool,
}

impl PublishInitCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;
        let registry_domain = client.get_warg_registry(self.name.namespace()).await?;

        let signing_key = self.common.signing_key(registry_domain.as_ref()).await?;
        match enqueue(&client, &self.name, |_| {
            std::future::ready(Ok(PublishEntry::Init))
        })
        .await?
        {
            Some(entry) => {
                let record_id = client
                    .publish_with_info(
                        &signing_key,
                        PublishInfo {
                            name: self.name.clone(),
                            head: None,
                            entries: vec![entry],
                        },
                    )
                    .await?;

                if self.no_wait {
                    println!("submitted record `{record_id}` for publishing");
                } else {
                    client
                        .wait_for_publish(&self.name, &record_id, DEFAULT_WAIT_INTERVAL)
                        .await?;

                    println!(
                        "published initialization of package `{name}`",
                        name = self.name,
                    );
                }
            }
            None => {
                println!(
                    "added initialization of package `{name}` to pending publish",
                    name = self.name
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
    /// The package name being published.
    #[clap(long, short, value_name = "PACKAGE")]
    pub name: PackageName,
    /// The version of the package being published.
    #[clap(long, short, value_name = "VERSION")]
    pub version: Version,
    /// The path to the package being published.
    #[clap(value_name = "PATH")]
    pub path: PathBuf,
    /// Whether to wait for the publish to complete.
    #[clap(long)]
    pub no_wait: bool,
}

impl PublishReleaseCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;
        let registry_domain = client.get_warg_registry(self.name.namespace()).await?;
        let signing_key = self.common.signing_key(registry_domain.as_ref()).await?;

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
                let record_id = client
                    .publish_with_info(
                        &signing_key,
                        PublishInfo {
                            name: self.name.clone(),
                            head: None,
                            entries: vec![entry],
                        },
                    )
                    .await?;

                if self.no_wait {
                    println!("submitted record `{record_id}` for publishing");
                } else {
                    client
                        .wait_for_publish(&self.name, &record_id, DEFAULT_WAIT_INTERVAL)
                        .await?;

                    println!(
                        "published version {version} of package `{name}`",
                        version = self.version,
                        name = self.name
                    );
                }
            }
            None => {
                println!(
                    "added release of version {version} for package `{name}` to pending publish",
                    version = self.version,
                    name = self.name
                );
            }
        }

        Ok(())
    }
}

/// Yank a package release from a warg registry.
#[derive(Args)]
#[clap(disable_version_flag = true)]
pub struct PublishYankCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The package name being yanked.
    #[clap(long, short, value_name = "PACKAGE")]
    pub name: PackageName,
    /// The version of the package being yanked.
    #[clap(long, short, value_name = "VERSION")]
    pub version: Version,
    /// Whether to wait for the publish to complete.
    #[clap(long)]
    pub no_wait: bool,
}

impl PublishYankCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        if !Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "`Yank` revokes a version, making it unavailable. It is permanent and cannot be reversed.
Yank `{version}` of `{package}`?",
                version = &self.version,
                package = &self.name,
            ))
            .default(false)
            .interact()?
        {
            println!("Aborted and did not yank.");
            return Ok(());
        }

        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;
        let registry_domain = client.get_warg_registry(self.name.namespace()).await?;
        let signing_key = self.common.signing_key(registry_domain.as_ref()).await?;

        let version = self.version.clone();
        match enqueue(&client, &self.name, move |_| async move {
            Ok(PublishEntry::Yank { version })
        })
        .await?
        {
            Some(entry) => {
                let record_id = client
                    .publish_with_info(
                        &signing_key,
                        PublishInfo {
                            name: self.name.clone(),
                            head: None,
                            entries: vec![entry],
                        },
                    )
                    .await?;

                if self.no_wait {
                    println!("submitted record `{record_id}` for publishing");
                } else {
                    client
                        .wait_for_publish(&self.name, &record_id, DEFAULT_WAIT_INTERVAL)
                        .await?;

                    println!(
                        "yanked version {version} of package `{name}`",
                        version = self.version,
                        name = self.name
                    );
                }
            }
            None => {
                println!(
                    "added yank of version {version} for package `{name}` to pending publish",
                    version = self.version,
                    name = self.name
                );
            }
        }

        Ok(())
    }
}

/// Publish a package to a warg registry.
#[derive(Args)]
#[clap(disable_version_flag = true)]
pub struct PublishGrantCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The package name.
    #[clap(long, short, value_name = "PACKAGE")]
    pub name: PackageName,
    /// The public key to grant permissions to.
    #[clap(value_name = "PUBLIC_KEY")]
    pub public_key: PublicKey,
    /// The permission(s) to grant.
    #[clap(
        long = "permission",
        value_delimiter = ',',
        default_value = "release,yank"
    )]
    pub permissions: Vec<Permission>,
    /// Whether to wait for the publish to complete.
    #[clap(long)]
    pub no_wait: bool,
}

impl PublishGrantCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;
        let registry_domain = client.get_warg_registry(self.name.namespace()).await?;
        let signing_key = self.common.signing_key(registry_domain.as_ref()).await?;

        match enqueue(&client, &self.name, |_| async {
            Ok(PublishEntry::Grant {
                key: self.public_key.clone(),
                permissions: self.permissions.clone(),
            })
        })
        .await?
        {
            Some(entry) => {
                let record_id = client
                    .publish_with_info(
                        &signing_key,
                        PublishInfo {
                            name: self.name.clone(),
                            head: None,
                            entries: vec![entry],
                        },
                    )
                    .await?;

                if self.no_wait {
                    println!("submitted record `{record_id}` for publishing");
                } else {
                    client
                        .wait_for_publish(&self.name, &record_id, DEFAULT_WAIT_INTERVAL)
                        .await?;

                    println!(
                        "granted ({permissions_str}) to key ID `{key_id}` for package `{name}`",
                        permissions_str = self.permissions.iter().join(","),
                        key_id = self.public_key.fingerprint(),
                        name = self.name
                    );
                }
            }
            None => {
                println!(
                    "added grant of ({permissions_str}) to key ID `{key_id}` for package `{name}` to pending publish",
                    permissions_str = self.permissions.iter().join(","),
                    key_id = self.public_key.fingerprint(),
                    name = self.name
                );
            }
        }

        Ok(())
    }
}

/// Publish a package to a warg registry.
#[derive(Args)]
#[clap(disable_version_flag = true)]
pub struct PublishRevokeCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The package name.
    #[clap(long, short, value_name = "PACKAGE")]
    pub name: PackageName,
    /// The key ID to revoke permissions from.
    #[clap(value_name = "KEY_ID")]
    pub key: KeyID,
    /// The permission(s) to revoke.
    #[clap(
        long = "permission",
        value_delimiter = ',',
        default_value = "release,yank"
    )]
    pub permissions: Vec<Permission>,
    /// Whether to wait for the publish to complete.
    #[clap(long)]
    pub no_wait: bool,
}

impl PublishRevokeCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;
        let registry_domain = client.get_warg_registry(self.name.namespace()).await?;
        let signing_key = self.common.signing_key(registry_domain.as_ref()).await?;

        match enqueue(&client, &self.name, |_| async {
            Ok(PublishEntry::Revoke {
                key_id: self.key.clone(),
                permissions: self.permissions.clone(),
            })
        })
        .await?
        {
            Some(entry) => {
                let record_id = client
                    .publish_with_info(
                        &signing_key,
                        PublishInfo {
                            name: self.name.clone(),
                            head: None,
                            entries: vec![entry],
                        },
                    )
                    .await?;

                if self.no_wait {
                    println!("submitted record `{record_id}` for publishing");
                } else {
                    client
                        .wait_for_publish(&self.name, &record_id, DEFAULT_WAIT_INTERVAL)
                        .await?;

                    println!(
                        "revoked ({permissions_str}) from key ID `{key_id}` for package `{name}`",
                        permissions_str = self.permissions.iter().join(","),
                        key_id = self.key,
                        name = self.name
                    );
                }
            }
            None => {
                println!(
                    "added revoke of ({permissions_str}) from key ID `{key_id}` for package `{name}` to pending publish",
                    permissions_str = self.permissions.iter().join(","),
                    key_id = self.key,
                    name = self.name
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
    /// The package name being published.
    #[clap(value_name = "PACKAGE")]
    pub name: PackageName,
}

impl PublishStartCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        match client.registry().load_publish().await? {
            Some(info) => bail!("a publish is already in progress for package `{name}`; use `publish abort` to abort the current publish", name = info.name),
            None => {
                client.registry().store_publish(Some(&PublishInfo {
                    name: self.name.clone(),
                    head: None,
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

        match client.registry().load_publish().await? {
            Some(info) => {
                println!(
                    "publishing package `{name}` with {count} record(s) to publish\n",
                    name = info.name,
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
                        PublishEntry::Yank { version } => {
                            println!("yank {version}")
                        }
                        PublishEntry::Grant { key, permissions } => println!(
                            "grant ({permissions_str}) to `{key_id}`",
                            permissions_str = permissions.iter().join(","),
                            key_id = key.fingerprint(),
                        ),
                        PublishEntry::Revoke {
                            key_id,
                            permissions,
                        } => println!(
                            "revoke ({permissions_str}) from `{key_id}`",
                            permissions_str = permissions.iter().join(","),
                        ),
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

        match client.registry().load_publish().await? {
            Some(info) => {
                client.registry().store_publish(None).await?;
                println!(
                    "aborted the pending publish for package `{name}`",
                    name = info.name
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
    /// Whether to wait for the publish to complete.
    #[clap(long)]
    pub no_wait: bool,
}

impl PublishSubmitCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        match client.registry().load_publish().await? {
            Some(info) => {
                println!(
                    "submitting publish for package `{name}`...",
                    name = info.name
                );

                let signing_key = self.common.signing_key(None).await?;
                let record_id = client.publish_with_info(&signing_key, info.clone()).await?;

                client.registry().store_publish(None).await?;

                if self.no_wait {
                    println!("submitted record `{record_id}` for publishing");
                } else {
                    client
                        .wait_for_publish(&info.name, &record_id, DEFAULT_WAIT_INTERVAL)
                        .await?;

                    for entry in &info.entries {
                        let name = &info.name;
                        match entry {
                            PublishEntry::Init => {
                                println!("published initialization of package `{name}`");
                            }
                            PublishEntry::Release { version, .. } => {
                                println!("published version {version} of package `{name}`");
                            }
                            PublishEntry::Yank { version } => {
                                println!("yanked version {version} of package `{name}`")
                            }
                            PublishEntry::Grant { key, permissions } => {
                                println!(
                                    "granted ({permissions_str}) to `{key_id}`",
                                    permissions_str = permissions.iter().join(","),
                                    key_id = key.fingerprint(),
                                )
                            }
                            PublishEntry::Revoke {
                                key_id,
                                permissions,
                            } => println!(
                                "revoked ({permissions_str}) from `{key_id}`",
                                permissions_str = permissions.iter().join(","),
                            ),
                        }
                    }
                }
            }
            None => bail!("no pending publish to submit"),
        }

        Ok(())
    }
}

/// Wait for a pending publish to complete.
#[derive(Args)]
pub struct PublishWaitCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// The name of the published package.
    #[clap(value_name = "PACKAGE")]
    pub name: PackageName,

    /// The identifier of the package record to wait for completion.
    #[clap(value_name = "RECORD")]
    pub record_id: AnyHash,
}

impl PublishWaitCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;
        let record_id = RecordId::from(self.record_id);

        println!(
            "waiting for record `{record_id} of package `{name}` to be published...",
            name = self.name
        );

        client
            .wait_for_publish(&self.name, &record_id, Duration::from_secs(1))
            .await?;

        println!(
            "record `{record_id} of package `{name}` has been published",
            name = self.name
        );

        Ok(())
    }
}
