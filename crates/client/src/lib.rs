//! A client library for Warg component registries.

#![deny(missing_docs)]

use crate::storage::PackageInfo;
use anyhow::{anyhow, Result};
use reqwest::{Body, IntoUrl};
use std::{borrow::Cow, collections::HashMap, path::PathBuf, time::Duration};
use storage::{
    ContentStorage, FileSystemContentStorage, FileSystemRegistryStorage, PublishInfo,
    RegistryStorage,
};
use thiserror::Error;
use warg_api::v1::{
    fetch::{FetchError, FetchLogsRequest, FetchLogsResponse},
    package::{PackageError, PackageRecord, PackageRecordState, PublishRecordRequest},
    proof::{ConsistencyRequest, InclusionRequest},
};
use warg_crypto::{
    hash::{DynHash, Hash, Sha256},
    signing,
};
use warg_protocol::{
    operator, package,
    registry::{LogId, LogLeaf, MapCheckpoint, RecordId},
    ProtoEnvelope, SerdeEnvelope, Version, VersionReq,
};

pub mod api;
mod config;
pub mod lock;
pub mod storage;
pub use self::config::*;

/// A client for a Warg registry.
pub struct Client<R, C> {
    registry: R,
    content: C,
    api: api::Client,
}

impl<R: RegistryStorage, C: ContentStorage> Client<R, C> {
    /// Creates a new client for the given URL, registry storage, and
    /// content storage.
    pub fn new(url: impl IntoUrl, registry: R, content: C) -> ClientResult<Self> {
        Ok(Self {
            registry,
            content,
            api: api::Client::new(url)?,
        })
    }

    /// Gets the URL of the client.
    pub fn url(&self) -> &str {
        self.api.url()
    }

    /// Gets the registry storage used by the client.
    pub fn registry(&self) -> &R {
        &self.registry
    }

    /// Gets the content storage used by the client.
    pub fn content(&self) -> &C {
        &self.content
    }

    /// Submits the publish information in client storage.
    ///
    /// If there's no publishing information in client storage, an error is returned.
    ///
    /// Returns the identifier of the record that was published.
    ///
    /// Use `wait_for_publish` to wait for the record to transition to the `published` state.
    pub async fn publish(&self, signing_key: &signing::PrivateKey) -> ClientResult<RecordId> {
        let info = self
            .registry
            .load_publish()
            .await?
            .ok_or(ClientError::NotPublishing)?;

        let res = self.publish_with_info(signing_key, info).await;
        self.registry.store_publish(None).await?;
        res
    }

    /// Submits the provided publish information.
    ///
    /// Any publish information in client storage is ignored.
    ///
    /// Returns the identifier of the record that was published.
    ///
    /// Use `wait_for_publish` to wait for the record to transition to the `published` state.
    pub async fn publish_with_info(
        &self,
        signing_key: &signing::PrivateKey,
        mut info: PublishInfo,
    ) -> ClientResult<RecordId> {
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
            .registry
            .load_package(&info.package)
            .await?
            .unwrap_or_else(|| PackageInfo::new(info.package.clone()));

        // If we're not initializing the package and a head was not explicitly specified,
        // updated to the latest checkpoint to get the latest known head.
        if !initializing && info.head.is_none() {
            self.update_checkpoint(&self.api.latest_checkpoint().await?, [&mut package])
                .await?;

            info.head = package.state.head().as_ref().map(|h| h.digest.clone());
        }

        match (initializing, info.head.is_some()) {
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

        let record = info.finalize(signing_key)?;

        let log_id = LogId::package_log::<Sha256>(&package.name);
        let record = self
            .api
            .publish_package_record(
                &log_id,
                PublishRecordRequest {
                    name: Cow::Borrowed(&package.name),
                    record: Cow::Owned(record.into()),
                    content_sources: Default::default(),
                },
            )
            .await
            .map_err(|e| {
                ClientError::translate_log_not_found(e, |id| {
                    if id == &log_id {
                        Some(package.name.clone())
                    } else {
                        None
                    }
                })
            })?;

        let missing = record.missing_content();
        if !missing.is_empty() {
            // Upload the missing content
            // TODO: parallelize this
            for digest in record.missing_content() {
                self.api
                    .upload_content(
                        &log_id,
                        &record.id,
                        digest,
                        Body::wrap_stream(self.content.load_content(digest).await?.ok_or_else(
                            || ClientError::ContentNotFound {
                                digest: digest.clone(),
                            },
                        )?),
                    )
                    .await
                    .map_err(|e| match e {
                        api::ClientError::Package(PackageError::Rejection(reason)) => {
                            ClientError::PublishRejected {
                                package: package.name.clone(),
                                record_id: record.id.clone(),
                                reason,
                            }
                        }
                        _ => e.into(),
                    })?;
            }
        }

        Ok(record.id)
    }

    /// Waits for a package record to transition to the `published` state.
    ///
    /// The `interval` is the amount of time to wait between checks.
    ///
    /// Returns an error if the package record was rejected.
    pub async fn wait_for_publish(
        &self,
        package: &str,
        record_id: &RecordId,
        interval: Duration,
    ) -> ClientResult<()> {
        let log_id = LogId::package_log::<Sha256>(package);
        let mut current = self.get_package_record(package, &log_id, record_id).await?;

        loop {
            match current.state {
                PackageRecordState::Sourcing { .. } => {
                    return Err(ClientError::PackageMissingContent);
                }
                PackageRecordState::Published { .. } => {
                    return Ok(());
                }
                PackageRecordState::Rejected { reason } => {
                    return Err(ClientError::PublishRejected {
                        package: package.to_string(),
                        record_id: record_id.clone(),
                        reason,
                    });
                }
                PackageRecordState::Processing => {
                    tokio::time::sleep(interval).await;
                    current = self.get_package_record(package, &log_id, record_id).await?;
                }
            }
        }
    }

    /// Updates every package log in client storage to the latest registry checkpoint.
    pub async fn update(&self) -> ClientResult<()> {
        tracing::info!("updating all packages to latest checkpoint");

        let mut updating = self.registry.load_packages().await?;
        self.update_checkpoint(&self.api.latest_checkpoint().await?, &mut updating)
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
                self.registry
                    .load_package(package)
                    .await?
                    .unwrap_or_else(|| PackageInfo::new(*package)),
            );
        }

        self.update_checkpoint(&self.api.latest_checkpoint().await?, &mut updating)
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
        let log_id = LogId::package_log::<Sha256>(&info.name);

        match info
            .state
            .releases()
            .filter_map(|r| {
                if !requirement.matches(&r.version) {
                    return None;
                }

                Some((&r.record_id, &r.version, r.content()?))
            })
            .max_by(|(_, a, ..), (_, b, ..)| a.cmp(b))
        {
            Some((record_id, version, digest)) => Ok(Some(PackageDownload {
                version: version.clone(),
                digest: digest.clone(),
                path: self.download_content(&log_id, record_id, digest).await?,
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
        let log_id = LogId::package_log::<Sha256>(&info.name);

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
            path: self
                .download_content(&log_id, &release.record_id, digest)
                .await?,
        })
    }

    async fn update_checkpoint(
        &self,
        checkpoint: &SerdeEnvelope<MapCheckpoint>,
        packages: impl IntoIterator<Item = &mut PackageInfo>,
    ) -> Result<(), ClientError> {
        let root: DynHash = Hash::<Sha256>::of(checkpoint.as_ref()).into();
        tracing::info!("updating to checkpoint `{root}`");

        let mut operator = self.registry.load_operator().await?.unwrap_or_default();

        // Map package identifiers to package logs that need to be updated
        let mut packages = packages
            .into_iter()
            .filter_map(|p| match &p.checkpoint {
                // Don't bother updating if the package is already at the specified checkpoint
                Some(c) if c == checkpoint => None,
                _ => Some((LogId::package_log::<Sha256>(&p.name), p)),
            })
            .inspect(|(_, p)| tracing::info!("package log `{name}` will be updated", name = p.name))
            .collect::<HashMap<_, _>>();

        if packages.is_empty() {
            return Ok(());
        }

        let mut last_known = packages
            .iter()
            .map(|(id, p)| {
                (
                    id.clone(),
                    p.state.head().as_ref().map(|h| h.digest.clone()),
                )
            })
            .collect::<HashMap<_, _>>();

        loop {
            let response: FetchLogsResponse = self
                .api
                .fetch_logs(FetchLogsRequest {
                    root: Cow::Borrowed(&root),
                    operator: operator
                        .state
                        .head()
                        .as_ref()
                        .map(|h| Cow::Borrowed(&h.digest)),
                    limit: None,
                    packages: Cow::Borrowed(&last_known),
                })
                .await
                .map_err(|e| {
                    ClientError::translate_log_not_found(e, |id| {
                        packages.get(id).map(|p| p.name.clone())
                    })
                })?;

            for record in response.operator {
                let record: ProtoEnvelope<operator::OperatorRecord> = record.try_into()?;
                operator
                    .state
                    .validate(&record)
                    .map_err(|inner| ClientError::OperatorValidationFailed { inner })?;
            }

            for (log_id, records) in response.packages {
                let package = packages.get_mut(&log_id).ok_or_else(|| {
                    anyhow!("received records for unknown package log `{log_id}`")
                })?;

                for record in records {
                    let record: ProtoEnvelope<package::PackageRecord> = record.try_into()?;
                    package.state.validate(&record).map_err(|inner| {
                        ClientError::PackageValidationFailed {
                            package: package.name.clone(),
                            inner,
                        }
                    })?;
                }

                // At this point, the package log should not be empty
                if package.state.head().is_none() {
                    return Err(ClientError::PackageLogEmpty {
                        package: package.name.clone(),
                    });
                }
            }

            if !response.more {
                break;
            }

            // Update the last known record ids for each package log
            for (id, record_id) in last_known.iter_mut() {
                *record_id = packages[id].state.head().as_ref().map(|h| h.digest.clone());
            }
        }

        // Prove inclusion for the current log heads
        let mut leafs = Vec::with_capacity(packages.len() + 1 /* for operator */);
        if let Some(head) = operator.state.head() {
            leafs.push(LogLeaf {
                log_id: LogId::operator_log::<Sha256>(),
                record_id: head.digest.clone(),
            });
        }

        for (log_id, package) in &packages {
            if let Some(head) = package.state.head() {
                leafs.push(LogLeaf {
                    log_id: log_id.clone(),
                    record_id: head.digest.clone(),
                });
            }
        }

        if !leafs.is_empty() {
            self.api
                .prove_inclusion(InclusionRequest {
                    checkpoint: Cow::Borrowed(checkpoint.as_ref()),
                    leafs: Cow::Borrowed(&leafs),
                })
                .await?;
        }

        if let Some(from) = self.registry.load_checkpoint().await? {
            self.api
                .prove_log_consistency(ConsistencyRequest {
                    from: Cow::Borrowed(&from.as_ref().log_root),
                    to: Cow::Borrowed(&checkpoint.as_ref().log_root),
                })
                .await?;
        }

        self.registry.store_operator(operator).await?;

        for package in packages.values_mut() {
            package.checkpoint = Some(checkpoint.clone());
            self.registry.store_package(package).await?;
        }

        self.registry.store_checkpoint(checkpoint).await?;

        Ok(())
    }

    async fn fetch_package(&self, name: &str) -> Result<PackageInfo, ClientError> {
        match self.registry.load_package(name).await? {
            Some(info) => {
                tracing::info!("log for package `{name}` already exists in storage");
                Ok(info)
            }
            None => {
                let mut info = PackageInfo::new(name);
                self.update_checkpoint(&self.api.latest_checkpoint().await?, [&mut info])
                    .await?;

                Ok(info)
            }
        }
    }

    async fn get_package_record(
        &self,
        package: &str,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> ClientResult<PackageRecord> {
        let record = self
            .api
            .get_package_record(log_id, record_id)
            .await
            .map_err(|e| {
                ClientError::translate_log_not_found(e, |id| {
                    if id == log_id {
                        Some(package.to_string())
                    } else {
                        None
                    }
                })
            })?;
        Ok(record)
    }

    async fn download_content(
        &self,
        log_id: &LogId,
        record_id: &RecordId,
        digest: &DynHash,
    ) -> Result<PathBuf, ClientError> {
        match self.content.content_location(digest) {
            Some(path) => {
                tracing::info!("content for digest `{digest}` already exists in storage");
                Ok(path)
            }
            None => {
                self.content
                    .store_content(
                        Box::pin(self.api.download_content(log_id, record_id, digest).await?),
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
}

/// A Warg registry client that uses the local file system to store
/// package logs and content.
pub type FileSystemClient = Client<FileSystemRegistryStorage, FileSystemContentStorage>;

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
        let StoragePaths {
            url,
            registries_dir,
            content_dir,
        } = config.storage_paths_for_url(url)?;

        let (packages, content) = match (
            FileSystemRegistryStorage::try_lock(registries_dir.clone())?,
            FileSystemContentStorage::try_lock(content_dir.clone())?,
        ) {
            (Some(packages), Some(content)) => (packages, content),
            (None, _) => return Ok(StorageLockResult::NotAcquired(registries_dir)),
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
        let StoragePaths {
            url,
            registries_dir,
            content_dir,
        } = config.storage_paths_for_url(url)?;
        Self::new(
            url,
            FileSystemRegistryStorage::lock(registries_dir)?,
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

    /// The operator failed validation.
    #[error("operator failed validation: {inner}")]
    OperatorValidationFailed {
        /// The validation error.
        inner: operator::ValidationError,
    },

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
        inner: package::ValidationError,
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
        /// The record identifier for the record that was rejected.
        record_id: RecordId,
        /// The reason it was rejected.
        reason: String,
    },

    /// The package is still missing content.
    #[error("the package is still missing content after all content was uploaded")]
    PackageMissingContent,

    /// An error occurred during an API operation.
    #[error(transparent)]
    Api(#[from] api::ClientError),

    /// An error occurred while performing a client operation.
    #[error("{0:?}")]
    Other(#[from] anyhow::Error),
}

impl ClientError {
    fn translate_log_not_found(
        e: api::ClientError,
        lookup: impl Fn(&LogId) -> Option<String>,
    ) -> Self {
        match &e {
            api::ClientError::Fetch(FetchError::LogNotFound(id))
            | api::ClientError::Package(PackageError::LogNotFound(id)) => {
                if let Some(package) = lookup(id) {
                    return Self::PackageDoesNotExist { package };
                }
            }
            _ => {}
        }

        Self::Api(e)
    }
}

/// Represents the result of a client operation.
pub type ClientResult<T> = Result<T, ClientError>;
