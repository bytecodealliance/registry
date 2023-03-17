//! A client library for Warg component registries.

#![deny(missing_docs)]

use crate::storage::PackageInfo;
use anyhow::Result;
use reqwest::Body;
use std::{collections::HashMap, path::PathBuf};
use storage::ClientStorage;
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
    api: api::Client,
}

impl<T: ClientStorage> Client<T> {
    /// Creates a new client with the given client storage.
    pub async fn new(storage: T) -> Result<Self, ClientError> {
        let api = storage
            .load_registry_info()
            .await?
            .map(|info| api::Client::new(info.url))
            .transpose()?
            .ok_or(ClientError::StorageNotInitialized)?;

        Ok(Self { storage, api })
    }

<<<<<<< HEAD
    pub async fn inform(&self, package: String) -> Result<(), ClientError> {
        let state = self.storage.load_package_state(&package).await?;
        println!("Versions");
        state.releases().for_each(|r| {
            println!(
                "{}.{}.{}",
                r.version.major, r.version.minor, r.version.patch
            )
        });
        Ok(())
    }

    async fn registry_info(&self) -> Result<RegistryInfo, ClientError> {
        match self.storage.load_registry_info().await? {
            Some(reg_info) => Ok(reg_info),
            None => Err(ClientError::RegistryNotSet),
        }
=======
    /// Gets the storage used by the client.
    pub fn storage(&self) -> &dyn ClientStorage {
        &self.storage
>>>>>>> main
    }

    /// Submits the publish information in client storage.
    ///
    /// If there's no publishing information in client storage, an error is returned.
    pub async fn publish(&mut self, signing_key: &signing::PrivateKey) -> Result<(), ClientError> {
        let info = self
            .storage
            .load_publish_info()
            .await?
            .ok_or(ClientError::NotPublishing)?;

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
            .storage
            .load_package_info(&info.package)
            .await?
            .unwrap_or_else(|| PackageInfo::new(info.package.clone()));

        // If we're not initializing the package, update it to the latest checkpoint to get the current head
        if !initializing {
            self.update_checkpoint(&self.api.latest_checkpoint().await?, [&mut package])
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

        let response = self
            .api
            .publish(&package.name, record.into(), sources)
            .await?;
        self.storage.store_publish_info(None).await?;

        // Finally, update the checkpoint again post-publish
        self.update_checkpoint(response.checkpoint.as_ref(), [&mut package])
            .await?;

        Ok(())
    }

    /// Updates every package log in client storage to the latest registry checkpoint.
    pub async fn update(&mut self) -> Result<(), ClientError> {
        tracing::info!("updating all packages to latest checkpoint");

        let mut updating = self.storage.load_packages().await?;
        self.update_checkpoint(&self.api.latest_checkpoint().await?, &mut updating)
            .await?;

        Ok(())
    }

    /// Inserts or updates the logs of the specified packages in client storage to
    /// the latest registry checkpoint.
    pub async fn upsert(&mut self, packages: &[&str]) -> Result<(), ClientError> {
        tracing::info!("updating specific packages to latest checkpoint");

        let mut updating = Vec::with_capacity(packages.len());
        for package in packages {
            updating.push(
                self.storage
                    .load_package_info(package)
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
        &mut self,
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

<<<<<<< HEAD
    async fn update_packages(
        &mut self,
        client: &api::Client,
        checkpoint: &SerdeEnvelope<MapCheckpoint>,
        packages: Vec<String>,
    ) -> Result<(), ClientError> {
        println!("The checkpoint to be hashed {:?}", checkpoint);
        let root: Hash<Sha256> = Hash::of(checkpoint.as_ref());
        let root: DynHash = root.into();
        println!("THE ROOT HASH {:?}", root);

        let mut validators = Vec::new();
        let mut heads = Vec::new();
        for name in packages {
            let state = self.storage.load_package_state(&name).await?;
            let head = state.head().as_ref().map(|head| head.digest.clone());
            validators.push((name.clone(), state));
            heads.push((name, head));
=======
        if packages.is_empty() {
            return Ok(());
>>>>>>> main
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

        self.api.prove_inclusion(checkpoint.as_ref(), heads).await?;

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
                let mut info = PackageInfo::new(name);
                self.update_checkpoint(&self.api.latest_checkpoint().await?, [&mut info])
                    .await?;

                Ok(info)
            }
        }
    }

    async fn download_content(&self, digest: &DynHash) -> Result<PathBuf, ClientError> {
        match self.storage.content_location(digest) {
            Some(path) => {
                tracing::info!("content for digest `{digest}` already exists in storage");
                Ok(path)
            }
            None => {
                tracing::info!("downloading content for digest `{digest}`");
                self.storage
                    .store_content(
                        Box::pin(self.api.download_content(digest).await?),
                        Some(digest),
                    )
                    .await?;

                self.storage
                    .content_location(digest)
                    .ok_or_else(|| ClientError::ContentNotFound {
                        digest: digest.clone(),
                    })
            }
        }
    }

    fn warg_url(package: &str) -> String {
        // TODO: currently this is required for parsing WIT packages
        // When the component model figures out what to store in extern descriptors, this
        // will likely be removed.
        format!("warg:///{id}", id = LogId::package_log::<Sha256>(package))
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
