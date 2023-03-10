//! A client library for Warg component registries.

#![deny(missing_docs)]

use crate::storage::PackageInfo;
use anyhow::Result;
use indexmap::IndexMap;
use reqwest::Body;
use std::path::PathBuf;
use storage::{ClientStorage, RegistryInfo};
use thiserror::Error;
use warg_api::{
    content::{ContentSource, ContentSourceKind},
    fetch::{FetchRequest, FetchResponse},
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
pub mod lock;
pub mod storage;

/// A client for a Warg registry.
pub struct Client<T> {
    storage: T,
    info: RegistryInfo,
    api: Option<api::Client>,
}

impl<T: ClientStorage> Client<T> {
    /// Creates a new client with the given client storage.
    pub async fn new(storage: T) -> Result<Self, ClientError> {
        let (info, api) = match storage.load_registry_info().await? {
            Some(info) => {
                let api = info.url().map(api::Client::new).transpose()?;
                (info, api)
            }
            None => return Err(ClientError::StorageNotInitialized),
        };

        Ok(Self { storage, info, api })
    }

    /// Gets the storage used by the client.
    pub fn storage(&self) -> &dyn ClientStorage {
        &self.storage
    }

    /// Submits the publish information in client storage.
    ///
    /// If there's no publishing information in client storage, an error is returned.
    ///
    /// Publishing to local registries is not supported.
    pub async fn publish(&mut self, signing_key: &signing::PrivateKey) -> Result<(), ClientError> {
        let api = self
            .api
            .as_ref()
            .ok_or(ClientError::OperationNotSupported)?;

        let info = self
            .storage
            .load_publish_info()
            .await?
            .ok_or(ClientError::NotPublishing)?;

        tracing::info!("publishing package `{package}`", package = info.package);
        let mut package = self
            .storage
            .load_package_info(&info.package)
            .await?
            .unwrap_or_else(|| PackageInfo::new(info.package.clone()));

        // If we're not initializing the package, update it to the latest checkpoint to get the current head
        if !info.init {
            self.update_checkpoint(&api.latest_checkpoint().await?, [&mut package])
                .await?;
        }

        match (info.init, package.state.head().is_some()) {
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

        if record.as_ref().entries.is_empty() {
            return Err(ClientError::NothingToPublish {
                package: package.name.clone(),
            });
        }

        let mut sources = Vec::with_capacity(contents.len());
        for content in contents {
            // Upload the content
            let url = api
                .upload_content(
                    &content,
                    Body::wrap_stream(self.storage.load_content(&content).await?.ok_or_else(
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

        let response = api.publish(&package.name, record.into(), sources).await?;

        self.storage.store_publish_info(None).await?;

        // Finally, update the checkpoint again post-publish
        self.update_checkpoint(response.checkpoint.as_ref(), [&mut package])
            .await?;

        Ok(())
    }

    /// Updates every package log in client storage to the latest registry checkpoint.
    ///
    /// This is a no-op for local registries.
    pub async fn update(&mut self) -> Result<(), ClientError> {
        tracing::info!("updating all packages to latest checkpoint");

        if let Some(api) = &self.api {
            let mut updating = self.storage.load_packages().await?;
            self.update_checkpoint(&api.latest_checkpoint().await?, &mut updating)
                .await?;
        }

        Ok(())
    }

    /// Inserts or updates the logs of the specified packages in client storage to
    /// the latest registry checkpoint.
    ///
    /// This is a no-op for local registries.
    pub async fn upsert(&mut self, packages: &[&str]) -> Result<(), ClientError> {
        tracing::info!("updating specific packages to latest checkpoint");

        if let Some(api) = &self.api {
            let mut updating = Vec::with_capacity(packages.len());
            for package in packages {
                updating.push(
                    self.storage
                        .load_package_info(package)
                        .await?
                        .unwrap_or_else(|| PackageInfo::new(*package)),
                );
            }

            self.update_checkpoint(&api.latest_checkpoint().await?, &mut updating)
                .await?;
        }

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
        &mut self,
        package: &str,
        requirement: &VersionReq,
    ) -> Result<Option<PackageDownload>, ClientError> {
        tracing::info!("downloading package `{package}` with requirement `{requirement}`");
        let versions = self.versions(package).await?;

        match versions
            .iter()
            .filter_map(|(version, digest)| {
                if !requirement.matches(version) {
                    return None;
                }

                Some((version, digest))
            })
            .max_by(|(a, ..), (b, ..)| a.cmp(b))
        {
            Some((version, digest)) => Ok(Some(PackageDownload {
                url: Self::warg_url(package),
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
        &mut self,
        package: &str,
        version: &Version,
    ) -> Result<PackageDownload, ClientError> {
        tracing::info!("downloading version {version} of package `{package}`");
        let versions = self.versions(package).await?;

        let digest =
            versions
                .get(version)
                .ok_or_else(|| ClientError::PackageVersionDoesNotExist {
                    version: version.clone(),
                    package: package.to_string(),
                })?;

        Ok(PackageDownload {
            url: Self::warg_url(package),
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
        let api = self
            .api
            .as_ref()
            .ok_or(ClientError::OperationNotSupported)?;

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
            .collect::<IndexMap<_, _>>();

        if packages.is_empty() {
            return Ok(());
        }

        let response: FetchResponse = api
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
                            ClientError::PackageValidationError {
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

        api.prove_inclusion(checkpoint.as_ref(), heads).await?;

        for package in packages.values_mut() {
            package.checkpoint = Some(checkpoint.clone());
            self.storage.store_package_info(package).await?;
        }

        Ok(())
    }

    async fn fetch_package(&self, name: &str) -> Result<PackageInfo, ClientError> {
        match self.storage.load_package_info(name).await? {
            Some(info) => {
                tracing::info!("log for package `{name}` already exists in storage");
                Ok(info)
            }
            None => {
                let api = self
                    .api
                    .as_ref()
                    .ok_or(ClientError::OperationNotSupported)?;

                let mut info = PackageInfo::new(name);
                self.update_checkpoint(&api.latest_checkpoint().await?, [&mut info])
                    .await?;

                Ok(info)
            }
        }
    }

    async fn download_content(&self, digest: &DynHash) -> Result<PathBuf, ClientError> {
        match self.storage.content_path(digest) {
            Some(path) => {
                tracing::info!("content for digest `{digest}` already exists in storage");
                Ok(path)
            }
            None => {
                tracing::info!("downloading content for digest `{digest}`");
                let api = self
                    .api
                    .as_ref()
                    .ok_or_else(|| ClientError::ContentNotFound {
                        digest: digest.clone(),
                    })?;

                self.storage
                    .store_content(Box::pin(api.download_content(digest).await?), Some(digest))
                    .await?;

                self.storage
                    .content_path(digest)
                    .ok_or_else(|| ClientError::ContentNotFound {
                        digest: digest.clone(),
                    })
            }
        }
    }

    async fn versions(&self, package: &str) -> Result<Versions, ClientError> {
        match &self.info {
            RegistryInfo::Remote { .. } => Ok(Versions::Remote(self.fetch_package(package).await?)),
            RegistryInfo::Local { packages, .. } => packages
                .get(package)
                .map(Versions::Local)
                .ok_or_else(|| ClientError::PackageDoesNotExist {
                    package: package.to_string(),
                }),
        }
    }

    fn warg_url(package: &str) -> String {
        // TODO: currently this is required for parsing WIT packages
        // When the component model figures out what to store in extern descriptors, this
        // will likely be removed.
        format!("warg:///{id}", id = LogId::package_log::<Sha256>(package))
    }
}

#[allow(clippy::large_enum_variant)]
enum Versions<'a> {
    Local(&'a IndexMap<Version, DynHash>),
    Remote(PackageInfo),
}

impl Versions<'_> {
    fn iter(&self) -> impl Iterator<Item = (&Version, &DynHash)> {
        match self {
            Self::Local(versions) => itertools::Either::Left(versions.iter()),
            Self::Remote(info) => itertools::Either::Right(
                info.state
                    .releases()
                    .filter_map(|r| Some((&r.version, r.content()?))),
            ),
        }
    }

    fn get(&self, version: &Version) -> Option<&DynHash> {
        match self {
            Self::Local(versions) => versions.get(version),
            Self::Remote(info) => info.state.release(version)?.content(),
        }
    }
}

/// Represents information about a downloaded package.
#[derive(Debug, Clone)]
pub struct PackageDownload {
    /// The url of the package.
    ///
    /// Currently this is a `warg://` URL for referencing in WIT packages.
    ///
    /// This field may be removed when the component model figures out what to store in extern
    /// descriptors.
    pub url: String,
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
    /// The storage provided to the client has not been initialized.
    #[error("the storage provided to the client has not been initialized")]
    StorageNotInitialized,

    /// The requested operation is not supported for this registry.
    #[error("the requested operation is not supported for this registry")]
    OperationNotSupported,

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
    #[error("package `{package}` failed validation: {inner}.")]
    PackageValidationError {
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

    /// An error occurred while communicating with the registry.
    #[error("an error occurred while communicating with registry {registry} ({status}): {body}")]
    ApiError {
        /// The registry server that returned the error.
        registry: String,
        /// The status code of the API error.
        status: u16,
        /// The response body.
        body: String,
    },

    /// An error occurred while proving the inclusion of a package in a registry checkpoint.
    #[error(transparent)]
    InclusionProof(#[from] warg_transparency::log::InclusionProofError),

    /// An error occurred while communicating with the registry.
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// An error occurred while performing a client operation
    #[error("{0:?}")]
    Other(#[from] anyhow::Error),
}
