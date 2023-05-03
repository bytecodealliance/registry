//! A client library for Warg component registries.

#![deny(missing_docs)]

use crate::storage::PackageInfo;
use anyhow::Result;
use reqwest::{Body, IntoUrl};
use std::{collections::HashMap, path::PathBuf, time::Duration};
use storage::{
    ContentStorage, FileSystemContentStorage, FileSystemPackageStorage, PackageStorage, PublishInfo,
};
use thiserror::Error;
use warg_api::{
    content::{ContentError, ContentSource, ContentSourceKind},
    fetch::{FetchError, FetchRequest, FetchResponse},
    package::{PackageError, PendingRecordResponse, RecordResponse},
    proof::ProofError,
};
use warg_crypto::{
    hash::{DynHash, Hash, Sha256},
    signing,
};
use warg_protocol::{
    package::{self, ValidationError},
    registry::{LogId, LogLeaf, MapCheckpoint},
    ProtoEnvelope, SerdeEnvelope, Version, VersionReq,
};

pub mod api;
mod config;
pub mod lock;
pub mod storage;
pub use self::config::*;

/// A client for a Warg registry.
pub struct Client<P, C> {
    packages: P,
    content: C,
    api: api::Client,
}

impl<P: PackageStorage, C: ContentStorage> Client<P, C> {
    /// Creates a new client for the given URL, package storage, and
    /// content storage.
    pub fn new(url: impl IntoUrl, packages: P, content: C) -> ClientResult<Self> {
        Ok(Self {
            packages,
            content,
            api: api::Client::new(url)?,
        })
    }

    /// Gets the URL of the client.
    pub fn url(&self) -> &str {
        self.api.url()
    }

    /// Gets the package storage used by the client.
    pub fn packages(&self) -> &P {
        &self.packages
    }

    /// Gets the content storage used by the client.
    pub fn content(&self) -> &C {
        &self.content
    }

    /// Submits the publish information in client storage.
    ///
    /// If there's no publishing information in client storage, an error is returned.
    pub async fn publish(&self, signing_key: &signing::PrivateKey) -> ClientResult<()> {
        let info = self
            .packages
            .load_publish()
            .await?
            .ok_or(ClientError::NotPublishing)?;

        let res = self.publish_with_info(signing_key, info).await;
        self.packages.store_publish(None).await?;
        res
    }

    /// Submits the provided publish information.
    ///
    /// Any publish information in client storage is ignored.
    pub async fn publish_with_info(
        &self,
        signing_key: &signing::PrivateKey,
        info: PublishInfo,
    ) -> ClientResult<()> {
        if info.entries.is_empty() {
            return Err(ClientError::NothingToPublish {
                package: info.package.clone(),
            });
        }

        let initializing = info.initializing();

        tracing::info!(
            "publishing {new}package `{package}`",
            package = info.package,
            new = if initializing { "new " } else { "" }
        );

        let mut package = self
            .packages
            .load_package(&info.package)
            .await?
            .unwrap_or_else(|| PackageInfo::new(info.package.clone()));

        // If we're not initializing the package, update it to the latest checkpoint to get the current head
        if !initializing {
            self.update_checkpoint(
                &self.api.latest_checkpoint().await?.checkpoint,
                [&mut package],
            )
            .await?;
        }

        match (initializing, package.state.head().is_some()) {
            (true, true) => {
                return Err(ClientError::CannotInitializePackage {
                    package: package.name,
                })
            }
            (false, false) => {
                return Err(ClientError::MustInitializePackage {
                    package: package.name,
                })
            }
            _ => (),
        }

        let (record, contents) = info.finalize(
            signing_key,
            package.state.head().as_ref().map(|h| h.digest.clone()),
        )?;

        let mut sources = Vec::with_capacity(contents.len());
        for content in contents {
            // Upload the content
            let url = self
                .api
                .upload_content(
                    &content,
                    Body::wrap_stream(self.content.load_content(&content).await?.ok_or_else(
                        || ClientError::ContentNotFound {
                            digest: content.clone(),
                        },
                    )?),
                )
                .await?;

            sources.push(ContentSource {
                digest: content.clone(),
                kind: ContentSourceKind::HttpAnonymous { url },
            });
        }

        let response = self
            .wait_for_publish(
                &package.name,
                self.api
                    .publish(&package.name, record.into(), sources)
                    .await?,
            )
            .await?;

        // Finally, update the checkpoint again post-publish
        self.update_checkpoint(&response.checkpoint, [&mut package])
            .await?;

        Ok(())
    }

    /// Updates every package log in client storage to the latest registry checkpoint.
    pub async fn update(&self) -> ClientResult<()> {
        tracing::info!("updating all packages to latest checkpoint");

        let mut updating = self.packages.load_packages().await?;
        self.update_checkpoint(
            &self.api.latest_checkpoint().await?.checkpoint,
            &mut updating,
        )
        .await?;

        Ok(())
    }

    /// Inserts or updates the logs of the specified packages in client storage to
    /// the latest registry checkpoint.
    pub async fn upsert(&self, packages: &[&str]) -> Result<(), ClientError> {
        tracing::info!("updating specific packages to latest checkpoint");

        let mut updating = Vec::with_capacity(packages.len());
        for package in packages {
            updating.push(
                self.packages
                    .load_package(package)
                    .await?
                    .unwrap_or_else(|| PackageInfo::new(*package)),
            );
        }

        self.update_checkpoint(
            &self.api.latest_checkpoint().await?.checkpoint,
            &mut updating,
        )
        .await?;

        Ok(())
    }

    /// Downloads the latest version of a package into client storage that
    /// satisfies the given version requirement.
    ///
    /// If the requested package log is not present in client storage, it
    /// will be fetched from the registry first.
    ///
    /// An error is returned if the package does not exist.
    ///
    /// If a version satisfying the requirement does not exist, `None` is
    /// returned.
    ///
    /// Returns the path within client storage of the package contents for
    /// the resolved version.
    pub async fn download(
        &self,
        package: &str,
        requirement: &VersionReq,
    ) -> Result<Option<PackageDownload>, ClientError> {
        tracing::info!("downloading package `{package}` with requirement `{requirement}`");
        let info = self.fetch_package(package).await?;

        match info
            .state
            .releases()
            .filter_map(|r| {
                if !requirement.matches(&r.version) {
                    return None;
                }

                Some((&r.version, r.content()?))
            })
            .max_by(|(a, ..), (b, ..)| a.cmp(b))
        {
            Some((version, digest)) => Ok(Some(PackageDownload {
                version: version.clone(),
                digest: digest.clone(),
                path: self.download_content(digest).await?,
            })),
            None => Ok(None),
        }
    }

    /// Downloads the specified version of a package into client storage.
    ///
    /// If the requested package log is not present in client storage, it
    /// will be fetched from the registry first.
    ///
    /// An error is returned if the package does not exist.
    ///
    /// Returns the path within client storage of the package contents for
    /// the specified version.
    pub async fn download_exact(
        &self,
        package: &str,
        version: &Version,
    ) -> Result<PackageDownload, ClientError> {
        tracing::info!("downloading version {version} of package `{package}`");
        let info = self.fetch_package(package).await?;

        let release =
            info.state
                .release(version)
                .ok_or_else(|| ClientError::PackageVersionDoesNotExist {
                    version: version.clone(),
                    package: package.to_string(),
                })?;

        let digest = release
            .content()
            .ok_or_else(|| ClientError::PackageVersionDoesNotExist {
                version: version.clone(),
                package: package.to_string(),
            })?;

        Ok(PackageDownload {
            version: version.clone(),
            digest: digest.clone(),
            path: self.download_content(digest).await?,
        })
    }

    async fn update_checkpoint(
        &self,
        checkpoint: &SerdeEnvelope<MapCheckpoint>,
        packages: impl IntoIterator<Item = &mut PackageInfo>,
    ) -> Result<(), ClientError> {
        tracing::info!(
            "updating to checkpoint `{log_root}|{map_root}`",
            log_root = checkpoint.as_ref().log_root,
            map_root = checkpoint.as_ref().map_root
        );

        let mut packages = packages
            .into_iter()
            .filter_map(|p| match &p.checkpoint {
                Some(c) if c == checkpoint => None,
                _ => Some((p.name.clone(), p)),
            })
            .inspect(|(n, _)| tracing::info!("log of package `{n}` will be updated"))
            .collect::<HashMap<_, _>>();

        if packages.is_empty() {
            return Ok(());
        }

        let response: FetchResponse = self
            .api
            .fetch_logs(FetchRequest {
                root: Hash::<Sha256>::of(checkpoint.as_ref()).into(),
                operator: None,
                packages: packages
                    .iter()
                    .map(|(name, package)| {
                        (
                            name.to_string(),
                            package.state.head().as_ref().map(|h| h.digest.clone()),
                        )
                    })
                    .collect(),
            })
            .await?;

        let mut heads = Vec::with_capacity(packages.len());
        for (name, records) in response.packages {
            match packages.get_mut(&name) {
                Some(package) => {
                    for record in records {
                        let record: ProtoEnvelope<package::PackageRecord> = record.try_into()?;
                        package.state.validate(&record).map_err(|inner| {
                            ClientError::PackageValidationFailed {
                                package: name.clone(),
                                inner,
                            }
                        })?;
                    }

                    if let Some(head) = package.state.head() {
                        heads.push(LogLeaf {
                            log_id: LogId::package_log::<Sha256>(&name),
                            record_id: head.digest.clone(),
                        });
                    } else {
                        return Err(ClientError::PackageLogEmpty {
                            package: name.clone(),
                        });
                    }
                }
                None => continue,
            }
        }

        self.api.prove_inclusion(checkpoint.as_ref(), heads).await?;

        for package in packages.values_mut() {
            package.checkpoint = Some(checkpoint.clone());
            self.packages.store_package(package).await?;
        }

        let old_checkpoint = self.packages.load_checkpoint().await?;
        if let Some(cp) = old_checkpoint {
            let old_cp = cp.as_ref().clone();
            let new_cp = checkpoint.as_ref().clone();
            self.api
                .prove_log_consistency(
                    old_cp.log_root,
                    new_cp.log_root,
                    old_cp.log_length,
                    new_cp.log_length,
                )
                .await?;
        }
        self.packages.store_checkpoint(checkpoint.clone()).await?;

        Ok(())
    }

    async fn fetch_package(&self, name: &str) -> Result<PackageInfo, ClientError> {
        match self.packages.load_package(name).await? {
            Some(info) => {
                tracing::info!("log for package `{name}` already exists in storage");
                Ok(info)
            }
            None => {
                let mut info = PackageInfo::new(name);
                self.update_checkpoint(
                    &self.api.latest_checkpoint().await?.checkpoint,
                    [&mut info],
                )
                .await?;

                Ok(info)
            }
        }
    }

    async fn download_content(&self, digest: &DynHash) -> Result<PathBuf, ClientError> {
        match self.content.content_location(digest) {
            Some(path) => {
                tracing::info!("content for digest `{digest}` already exists in storage");
                Ok(path)
            }
            None => {
                tracing::info!("downloading content for digest `{digest}`");
                self.content
                    .store_content(
                        Box::pin(self.api.download_content(digest).await?),
                        Some(digest),
                    )
                    .await?;

                self.content
                    .content_location(digest)
                    .ok_or_else(|| ClientError::ContentNotFound {
                        digest: digest.clone(),
                    })
            }
        }
    }

    async fn wait_for_publish(
        &self,
        name: &str,
        mut response: PendingRecordResponse,
    ) -> ClientResult<RecordResponse> {
        loop {
            match response {
                PendingRecordResponse::Published { record_url } => {
                    return Ok(self.api.get_package_record(&record_url).await?);
                }
                PendingRecordResponse::Rejected { reason } => {
                    return Err(ClientError::PublishRejected {
                        package: name.to_string(),
                        reason,
                    });
                }
                PendingRecordResponse::Processing { status_url } => {
                    // TODO: make the wait configurable
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    response = self.api.get_pending_package_record(&status_url).await?;
                }
            }
        }
    }
}

/// A Warg registry client that uses the local file system to store
/// package logs and content.
pub type FileSystemClient = Client<FileSystemPackageStorage, FileSystemContentStorage>;

/// A result of an attempt to lock client storage.
pub enum StorageLockResult<T> {
    /// The storage lock was acquired.
    Acquired(T),
    /// The storage lock was not acquired for the specified directory.
    NotAcquired(PathBuf),
}

impl FileSystemClient {
    /// Attempts to create a client for the given registry URL.
    ///
    /// If the URL is `None`, the default URL is used; if there is no default
    /// URL, an error is returned.
    ///
    /// If a lock cannot be acquired for a storage directory, then
    /// `NewClientResult::Blocked` is returned with the path to the
    /// directory that could not be locked.
    pub fn try_new_with_config(
        url: Option<&str>,
        config: &Config,
    ) -> Result<StorageLockResult<Self>, ClientError> {
        let (url, packages_dir, content_dir) = config.storage_paths_for_url(url)?;

        let (packages, content) = match (
            FileSystemPackageStorage::try_lock(packages_dir.clone())?,
            FileSystemContentStorage::try_lock(content_dir.clone())?,
        ) {
            (Some(packages), Some(content)) => (packages, content),
            (None, _) => return Ok(StorageLockResult::NotAcquired(packages_dir)),
            (_, None) => return Ok(StorageLockResult::NotAcquired(content_dir)),
        };

        Ok(StorageLockResult::Acquired(Self::new(
            url, packages, content,
        )?))
    }

    /// Creates a client for the given registry URL.
    ///
    /// If the URL is `None`, the default URL is used; if there is no default
    /// URL, an error is returned.
    ///
    /// This method blocks if storage locks cannot be acquired.
    pub fn new_with_config(url: Option<&str>, config: &Config) -> Result<Self, ClientError> {
        let (url, packages_dir, content_dir) = config.storage_paths_for_url(url)?;
        Self::new(
            url,
            FileSystemPackageStorage::lock(packages_dir)?,
            FileSystemContentStorage::lock(content_dir)?,
        )
    }
}

/// Represents information about a downloaded package.
#[derive(Debug, Clone)]
pub struct PackageDownload {
    /// The package version that was downloaded.
    pub version: Version,
    /// The digest of the package contents.
    pub digest: DynHash,
    /// The path to the downloaded package contents.
    pub path: PathBuf,
}

/// Represents an error returned by Warg registry clients.
#[derive(Debug, Error)]
pub enum ClientError {
    /// No default registry server URL is configured.
    #[error("no default registry server URL is configured")]
    NoDefaultUrl,

    /// The package already exists and cannot be initialized.
    #[error("package `{package}` already exists and cannot be initialized")]
    CannotInitializePackage {
        /// The name of the package that exists.
        package: String,
    },

    /// The package must be initialized before publishing.
    #[error("package `{package}` must be initialized before publishing")]
    MustInitializePackage {
        /// The name of the package that must be initialized.
        package: String,
    },

    /// There is no publish operation in progress.
    #[error("there is no publish operation in progress")]
    NotPublishing,

    /// The package has no records to publish.
    #[error("package `{package}` has no records to publish")]
    NothingToPublish {
        /// The name of the package that has no publish operations.
        package: String,
    },

    /// The package does not exist.
    #[error("package `{package}` does not exist")]
    PackageDoesNotExist {
        /// The name of the missing package.
        package: String,
    },

    /// The package version does not exist.
    #[error("version `{version}` of package `{package}` does not exist")]
    PackageVersionDoesNotExist {
        /// The missing version of the package.
        version: Version,
        /// The package with the missing version.
        package: String,
    },

    /// The package failed validation.
    #[error("package `{package}` failed validation: {inner}")]
    PackageValidationFailed {
        /// The package that failed validation.
        package: String,
        /// The validation error.
        inner: ValidationError,
    },

    /// Content was not found during a publish operation.
    #[error("content with digest `{digest}` was not found in client storage")]
    ContentNotFound {
        /// The digest of the missing content.
        digest: DynHash,
    },

    /// The package log is empty and cannot be validated.
    #[error("package log is empty and cannot be validated")]
    PackageLogEmpty {
        /// The name of the package with an empty package log.
        package: String,
    },

    /// A publish operation was rejected.
    #[error("the publishing of package `{package}` was rejected due to: {reason}")]
    PublishRejected {
        /// The package that was rejected.
        package: String,
        /// The reason it was rejected.
        reason: String,
    },

    /// An error occurred while communicating with the content service.
    #[error(transparent)]
    Content(#[from] ContentError),

    /// An error occurred while communicating with the fetch service.
    #[error(transparent)]
    Fetch(#[from] FetchError),

    /// An error occurred while communicating with the package service.
    #[error(transparent)]
    Package(#[from] PackageError),

    /// An error occurred while communicating with the proof service.
    #[error(transparent)]
    Proof(#[from] ProofError),

    /// An error occurred while performing a client operation.
    #[error("{0:?}")]
    Other(#[from] anyhow::Error),
}

/// Represents the result of a client operation.
pub type ClientResult<T> = Result<T, ClientError>;
