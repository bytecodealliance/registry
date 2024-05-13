//! A client library for Warg component registries.

#![deny(missing_docs)]
use crate::storage::PackageInfo;

use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use futures_util::{Stream, StreamExt, TryStreamExt};
use indexmap::IndexMap;
use reqwest::{Body, IntoUrl};
use secrecy::Secret;
use semver::{Version, VersionReq};
use std::cmp::Ordering;
use std::fs;
use std::str::FromStr;
use std::{borrow::Cow, path::PathBuf, time::Duration};
use storage::{
    ContentStorage, FileSystemContentStorage, FileSystemNamespaceMapStorage,
    FileSystemRegistryStorage, NamespaceMapStorage, PublishInfo, RegistryDomain, RegistryStorage,
};
use thiserror::Error;
use tokio_util::io::ReaderStream;
use warg_api::v1::{
    fetch::{FetchError, FetchLogsRequest},
    package::{
        MissingContent, PackageError, PackageRecord, PackageRecordState, PublishRecordRequest,
        UploadEndpoint,
    },
    proof::{ConsistencyRequest, InclusionRequest},
};
use warg_crypto::hash::Sha256;
use warg_crypto::{hash::AnyHash, signing, Encode, Signable};
use warg_protocol::package::ReleaseState;
use warg_protocol::{
    operator, package,
    registry::{LogId, LogLeaf, PackageName, RecordId, RegistryLen, TimestampedCheckpoint},
    PublishedProtoEnvelope,
};
use wasm_compose::graph::{CompositionGraph, EncodeOptions, ExportIndex, InstanceId};

#[cfg(feature = "keyring")]
pub mod keyring;

pub mod api;
mod config;
/// Tools for locking and bundling components
pub mod depsolve;
use depsolve::{Bundler, LockListBuilder};
/// Tools for semver
pub mod version_util;
use version_util::{kindless_name, locked_package, versioned_package, Import, ImportKind};
pub mod lock;
mod registry_url;
pub mod storage;
pub use self::config::*;
pub use self::registry_url::RegistryUrl;

const DEFAULT_WAIT_INTERVAL: Duration = Duration::from_secs(1);

/// A client for a Warg registry.
pub struct Client<R, C, N>
where
    R: RegistryStorage,
    C: ContentStorage,
    N: NamespaceMapStorage,
{
    registry: R,
    content: C,
    namespace_map: N,
    api: api::Client,
    ignore_federation_hints: bool,
    auto_accept_federation_hints: bool,
    disable_interactive: bool,
}

impl<R: RegistryStorage, C: ContentStorage, N: NamespaceMapStorage> Client<R, C, N> {
    /// Creates a new client for the given URL, registry storage, and
    /// content storage.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        url: impl IntoUrl,
        registry: R,
        content: C,
        namespace_map: N,
        auth_token: Option<Secret<String>>,
        ignore_federation_hints: bool,
        auto_accept_federation_hints: bool,
        disable_interactive: bool,
    ) -> ClientResult<Self> {
        let api = api::Client::new(url, auth_token)?;
        Ok(Self {
            registry,
            content,
            namespace_map,
            api,
            ignore_federation_hints,
            auto_accept_federation_hints,
            disable_interactive,
        })
    }

    /// Gets the URL of the client.
    pub fn url(&self) -> &RegistryUrl {
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

    /// Gets the namespace map
    pub fn namespace_map(&self) -> &N {
        &self.namespace_map
    }

    /// Get warg registry domain.
    pub async fn get_warg_registry(
        &self,
        namespace: &str,
    ) -> Result<Option<RegistryDomain>, ClientError> {
        let operator = self
            .registry()
            .load_operator(Some(&RegistryDomain::from_str(namespace)?))
            .await?;
        if let Some(op) = operator {
            match op.state.namespace_state(namespace) {
                Some(warg_protocol::operator::NamespaceState::Imported { registry }) => {
                    return Ok(Some(RegistryDomain::from_str(registry)?));
                }
                Some(warg_protocol::operator::NamespaceState::Defined) => {
                    return Ok(None);
                }
                _ => (),
            }
        };
        let nm_map = self.namespace_map.load_namespace_map().await?;
        Ok(nm_map.and_then(|nm_map| {
            nm_map
                .get(namespace)
                .map(|domain| RegistryDomain::from_str(domain).unwrap())
        }))
    }

    /// Stores namespace mapping in local storage
    pub async fn store_namespace(
        &self,
        namespace: String,
        registry_domain: RegistryDomain,
    ) -> Result<()> {
        self.namespace_map
            .store_namespace(namespace, registry_domain)
            .await?;
        Ok(())
    }

    /// Resets the namespace map
    pub async fn reset_namespaces(&self) -> Result<()> {
        self.namespace_map.reset_namespaces().await?;
        Ok(())
    }

    /// Reset client storage for the registry.
    pub async fn reset_registry(&self) -> ClientResult<()> {
        tracing::info!("resetting registry local state");
        self.registry
            .reset(true)
            .await
            .or(Err(ClientError::ResettingRegistryLocalStateFailed))
    }

    /// Clear client content cache.
    pub async fn clear_content_cache(&self) -> ClientResult<()> {
        tracing::info!("removing content cache");
        self.content
            .clear()
            .await
            .or(Err(ClientError::ClearContentCacheFailed))
    }

    /// Locks component
    pub async fn lock_component(&self, info: &PackageInfo) -> ClientResult<Vec<u8>> {
        let mut builder = LockListBuilder::default();
        builder.build_list(self, info).await?;
        let top = Import {
            name: format!("{}:{}", info.name.namespace(), info.name.name()),
            req: VersionReq::STAR,
            kind: ImportKind::Unlocked,
        };
        builder.lock_list.insert(top);
        let mut composer = CompositionGraph::new();
        let mut handled = IndexMap::<String, InstanceId>::new();
        for package in builder.lock_list {
            let name = package.name.clone();
            let version = package.req;
            let id = PackageName::new(name)?;
            let info = self
                .registry()
                .load_package(self.get_warg_registry(id.namespace()).await?.as_ref(), &id)
                .await?;
            if let Some(inf) = info {
                let release = if version != VersionReq::STAR {
                    inf.state
                        .releases()
                        .filter(|r| version.matches(&r.version))
                        .last()
                } else {
                    inf.state.releases().last()
                };

                if let Some(r) = release {
                    let state = &r.state;
                    if let ReleaseState::Released { content } = state {
                        let locked_package = locked_package(&package.name, r, content);
                        let path = self.content().content_location(content);
                        if let Some(p) = path {
                            let bytes = fs::read(&p).map_err(|_| ClientError::ContentNotFound {
                                digest: content.clone(),
                            })?;

                            let read_digest =
                                AnyHash::from_str(&format!("sha256:{}", sha256::digest(bytes)))
                                    .unwrap();
                            if content != &read_digest {
                                return Err(ClientError::IncorrectContent {
                                    digest: read_digest,
                                    expected: content.clone(),
                                });
                            }
                            let component =
                                wasm_compose::graph::Component::from_file(&locked_package, p)?;
                            let component_id = if let Some((id, _)) =
                                composer.get_component_by_name(&locked_package)
                            {
                                id
                            } else {
                                composer.add_component(component)?
                            };
                            let instance_id = composer.instantiate(component_id)?;
                            let added = composer.get_component(component_id);
                            handled.insert(versioned_package(&package.name, version), instance_id);
                            let mut args = Vec::new();
                            if let Some(added) = added {
                                for (index, name, _) in added.imports() {
                                    let iid = handled.get(kindless_name(name));
                                    if let Some(arg) = iid {
                                        args.push((arg, index));
                                    }
                                }
                            }
                            for arg in args {
                                composer.connect(
                                    *arg.0,
                                    None::<ExportIndex>,
                                    instance_id,
                                    arg.1,
                                )?;
                            }
                        }
                    }
                }
            }
        }
        let final_name = &format!("{}:{}", info.name.namespace(), &info.name.name());
        let id = handled.get(final_name);
        let options = EncodeOptions {
            export: id.copied(),
            ..Default::default()
        };
        let locked = composer.encode(options)?;
        fs::write("./locked.wasm", locked.as_slice()).map_err(|e| ClientError::Other(e.into()))?;
        Ok(locked)
    }

    /// Bundles component
    pub async fn bundle_component(&self, info: &PackageInfo) -> ClientResult<Vec<u8>> {
        let mut bundler = Bundler::new(self);
        let path = PathBuf::from("./locked.wasm");
        let locked = if !path.is_file() {
            self.lock_component(info).await?
        } else {
            fs::read("./locked.wasm").map_err(|e| ClientError::Other(e.into()))?
        };
        let bundled = bundler.parse(&locked).await?;
        fs::write("./bundled.wasm", bundled.as_slice())
            .map_err(|e| ClientError::Other(e.into()))?;
        Ok(bundled.as_slice().to_vec())
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
        publish_info: PublishInfo,
    ) -> ClientResult<RecordId> {
        if publish_info.entries.is_empty() {
            return Err(ClientError::NothingToPublish {
                name: publish_info.name.clone(),
            });
        }

        tracing::info!(
            "publishing {new}package `{name}`",
            name = publish_info.name,
            new = if publish_info.initializing() {
                "new "
            } else {
                ""
            }
        );
        tracing::debug!("entries: {:?}", publish_info.entries);

        let mut accepted_prompt_to_initialize = false;

        let mut init_record_id: Option<RecordId> = None;

        let (package, record) = loop {
            let mut info = publish_info.clone();

            let mut initializing = info.initializing();

            let package = match self.fetch_package(&info.name).await {
                Ok(package) => {
                    if initializing {
                        return Err(ClientError::CannotInitializePackage {
                            name: package.name,
                            init_record_id,
                        });
                    } else if info.head.is_none() {
                        // If we're not initializing the package and a head was not explicitly specified,
                        // set to the latest known head.
                        info.head = package.state.head().as_ref().map(|h| h.digest.clone());
                    }
                    package
                }
                Err(ClientError::PackageDoesNotExist {
                    name,
                    has_auth_token,
                }) => {
                    if !initializing {
                        if self.disable_interactive || cfg!(not(feature = "cli-interactive")) {
                            return Err(ClientError::MustInitializePackage {
                                name,
                                has_auth_token,
                            });
                        }

                        #[cfg(feature = "cli-interactive")]
                        {
                            use crate::storage::PublishEntry;
                            use dialoguer::{theme::ColorfulTheme, Confirm};

                            if accepted_prompt_to_initialize
                                || Confirm::with_theme(&ColorfulTheme::default())
                                    .with_prompt(format!(
                                        "Package `{package_name}` was not found.
If it exists, you may not have access.
Attempt to create `{package_name}` and publish the release y/N\n",
                                        package_name = &info.name,
                                    ))
                                    .default(false)
                                    .interact()
                                    .unwrap()
                            {
                                info.entries.insert(0, PublishEntry::Init);
                                initializing = true;
                                accepted_prompt_to_initialize = true;
                            } else {
                                return Err(ClientError::MustInitializePackage {
                                    name,
                                    has_auth_token,
                                });
                            }
                        }
                    }
                    PackageInfo::new(info.name.clone())
                }
                err => err?,
            };
            let registry_domain = self.get_warg_registry(package.name.namespace()).await?;

            let log_id = LogId::package_log::<Sha256>(&package.name);
            let record = info.finalize(signing_key)?;
            let record_id = RecordId::package_record::<Sha256>(&record);
            let record = match self
                .api
                .publish_package_record(
                    registry_domain.as_ref(),
                    &log_id,
                    PublishRecordRequest {
                        package_name: Cow::Borrowed(&package.name),
                        record: Cow::Owned(record.into()),
                        content_sources: Default::default(),
                    },
                )
                .await
            {
                Ok(record) => Ok(record),
                Err(api::ClientError::Package(PackageError::Rejection(reason))) => {
                    Err(ClientError::PublishRejected {
                        name: package.name.clone(),
                        reason,
                        record_id,
                    })
                }
                Err(api::ClientError::Package(PackageError::Unauthorized(reason))) => {
                    Err(ClientError::Unauthorized(reason))
                }
                Err(api::ClientError::Package(PackageError::ConflictPendingPublish(
                    pending_record_id,
                ))) => {
                    // conflicting pending publish succeeds,
                    tracing::info!("waiting for conflicting publish to complete");
                    // check registry for federated namespace mapping, if initializing
                    if initializing {
                        match self.fetch_package(&package.name).await {
                            Ok(_) => {}
                            // may not exist until conflicting publish completes
                            Err(ClientError::PackageDoesNotExist { .. }) => {}
                            Err(err) => return Err(err),
                        }
                        init_record_id = Some(pending_record_id.clone());
                    }
                    self.wait_for_publish(&package.name, &pending_record_id, DEFAULT_WAIT_INTERVAL)
                        .await
                        .map_err(|err| match err {
                            ClientError::PackageMissingContent => {
                                ClientError::ConflictPendingPublish {
                                    name: package.name.clone(),
                                    record_id,
                                    pending_record_id,
                                }
                            }
                            err => err,
                        })?;

                    continue;
                }
                Err(e) => Err(ClientError::translate_log_not_found(
                    e,
                    self.api.auth_token().is_some(),
                    |id| {
                        if id == &log_id {
                            Some(package.name.clone())
                        } else {
                            None
                        }
                    },
                )),
            }?;

            break (package, record);
        };

        // TODO: parallelize this
        for (digest, MissingContent { upload }) in record.missing_content() {
            // Upload the missing content, if the registry supports it
            let Some(UploadEndpoint::Http {
                method,
                url,
                headers,
            }) = upload.first()
            else {
                continue;
            };

            self.api
                .upload_content(
                    method,
                    url,
                    headers,
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
                            name: package.name.clone(),
                            record_id: record.record_id.clone(),
                            reason,
                        }
                    }
                    api::ClientError::Package(PackageError::Unauthorized(reason)) => {
                        ClientError::Unauthorized(reason)
                    }
                    _ => e.into(),
                })?;
        }

        Ok(record.record_id)
    }

    /// Waits for a package record to transition to the `published` state.
    ///
    /// The `interval` is the amount of time to wait between checks.
    ///
    /// Returns an error if the package record was rejected.
    pub async fn wait_for_publish(
        &self,
        package: &PackageName,
        record_id: &RecordId,
        interval: Duration,
    ) -> ClientResult<()> {
        let registry_domain = self.get_warg_registry(package.namespace()).await?;
        let log_id = LogId::package_log::<Sha256>(package);
        let mut current = self
            .get_package_record(registry_domain.as_ref(), package, &log_id, record_id)
            .await?;

        loop {
            match current.state {
                PackageRecordState::Sourcing { .. } => {
                    return Err(ClientError::PackageMissingContent);
                }
                PackageRecordState::Published { .. } => {
                    self.fetch_package(package).await?;
                    return Ok(());
                }
                PackageRecordState::Rejected { reason } => {
                    return Err(ClientError::PublishRejected {
                        name: package.clone(),
                        record_id: record_id.clone(),
                        reason,
                    });
                }
                PackageRecordState::Processing => {
                    tokio::time::sleep(interval).await;
                    current = self
                        .get_package_record(registry_domain.as_ref(), package, &log_id, record_id)
                        .await?;
                }
            }
        }
    }

    /// Updates all package logs in client registry storage to the latest registry checkpoint.
    pub async fn update(&self) -> ClientResult<()> {
        tracing::info!("updating downloaded package logs");

        for mut packages in self.registry.load_all_packages().await?.into_values() {
            self.update_checkpoints(&mut packages).await?;
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
        &self,
        package: &PackageName,
        requirement: &VersionReq,
    ) -> Result<Option<PackageDownload>, ClientError> {
        let info = self.package(package).await?;

        let registry_domain = self.get_warg_registry(package.namespace()).await?;

        tracing::debug!(
            package = package.as_ref(),
            version_requirement = requirement.to_string(),
            registry_header = ?registry_domain,
            "downloading",
        );

        match info.state.find_latest_release(requirement) {
            Some(release) => {
                let digest = release
                    .content()
                    .context("invalid state: not yanked but missing content")?
                    .clone();
                let path = self
                    .download_content(registry_domain.as_ref(), &digest)
                    .await?;
                Ok(Some(PackageDownload {
                    version: release.version.clone(),
                    digest,
                    path,
                }))
            }
            None => Ok(None),
        }
    }

    /// Downloads the latest version of a package.
    ///
    /// If the requested package log is not present in client storage, it
    /// will be fetched from the registry first.
    ///
    /// An error is returned if the package does not exist.
    ///
    /// If a version satisfying the requirement does not exist, `None` is
    /// returned.
    pub async fn download_as_stream(
        &self,
        package: &PackageName,
        requirement: &VersionReq,
    ) -> Result<Option<(PackageDownloadInfo, impl Stream<Item = Result<Bytes>>)>, ClientError> {
        let info = self.package(package).await?;

        let registry_domain = self.get_warg_registry(package.namespace()).await?;

        tracing::debug!(
            package = package.as_ref(),
            version_requirement = requirement.to_string(),
            registry_header = ?registry_domain,
            "downloading",
        );

        match info.state.find_latest_release(requirement) {
            Some(release) => {
                let digest = release
                    .content()
                    .context("invalid state: not yanked but missing content")?
                    .clone();
                let stream = self
                    .download_content_stream(registry_domain.as_ref(), &digest)
                    .await?;
                Ok(Some((
                    PackageDownloadInfo {
                        version: release.version.clone(),
                        digest,
                    },
                    stream,
                )))
            }
            None => Ok(None),
        }
    }

    /// Downloads the specified version of a package into client storage.
    ///
    /// If the requested package log is not present in client storage, it
    /// will be fetched from the registry first.
    ///
    /// An error is returned if the package or version does not exist.
    ///
    /// Returns the path within client storage of the package contents for
    /// the specified version.
    pub async fn download_exact(
        &self,
        package: &PackageName,
        version: &Version,
    ) -> Result<PackageDownload, ClientError> {
        let info = self.package(package).await?;

        let registry_domain = self.get_warg_registry(package.namespace()).await?;

        tracing::debug!(
            package = package.as_ref(),
            version = version.to_string(),
            registry_header = ?registry_domain,
            "downloading exact version",
        );

        let release =
            info.state
                .release(version)
                .ok_or_else(|| ClientError::PackageVersionDoesNotExist {
                    version: version.clone(),
                    name: package.clone(),
                })?;

        let digest = release
            .content()
            .ok_or_else(|| ClientError::PackageVersionDoesNotExist {
                version: version.clone(),
                name: package.clone(),
            })?;

        Ok(PackageDownload {
            version: version.clone(),
            digest: digest.clone(),
            path: self
                .download_content(registry_domain.as_ref(), digest)
                .await?,
        })
    }

    /// Downloads the specified version of a package.
    ///
    /// If the requested package log is not present in client storage, it
    /// will be fetched from the registry first.
    ///
    /// An error is returned if the package or version does not exist.
    pub async fn download_exact_as_stream(
        &self,
        package: &PackageName,
        version: &Version,
    ) -> Result<(PackageDownloadInfo, impl Stream<Item = Result<Bytes>>), ClientError> {
        let info = self.package(package).await?;

        let registry_domain = self.get_warg_registry(package.namespace()).await?;

        tracing::debug!(
            package = package.as_ref(),
            version = version.to_string(),
            registry_header = ?registry_domain,
            "downloading exact version",
        );

        let release =
            info.state
                .release(version)
                .ok_or_else(|| ClientError::PackageVersionDoesNotExist {
                    version: version.clone(),
                    name: package.clone(),
                })?;

        let digest = release
            .content()
            .ok_or_else(|| ClientError::PackageVersionDoesNotExist {
                version: version.clone(),
                name: package.clone(),
            })?;

        Ok((
            PackageDownloadInfo {
                version: version.clone(),
                digest: digest.clone(),
            },
            self.download_content_stream(registry_domain.as_ref(), digest)
                .await?,
        ))
    }

    async fn update_packages_and_return_federated_packages<'a>(
        &self,
        registry_domain: Option<&RegistryDomain>,
        packages: impl IntoIterator<Item = &'a mut PackageInfo>,
    ) -> Result<IndexMap<Option<RegistryDomain>, Vec<&'a mut PackageInfo>>, ClientError> {
        let ts_checkpoint = self.api.latest_checkpoint(registry_domain).await?;
        let checkpoint = &ts_checkpoint.as_ref().checkpoint;

        tracing::debug!(
            log_length = checkpoint.log_length,
            registry_header = ?registry_domain,
            "updating to checkpoint",
        );

        // operator log info
        let mut operator = self
            .registry
            .load_operator(registry_domain)
            .await?
            .unwrap_or_default();

        // map package names to package logs that need to be updated
        let mut packages = packages
            .into_iter()
            .filter_map(|p| match &p.checkpoint {
                // Don't bother updating if the package is already at the specified checkpoint
                // If `registry` field is not set, then update.
                Some(c) if p.registry.is_some() && c == checkpoint => None,
                _ => Some((LogId::package_log::<Sha256>(&p.name), p)),
            })
            .inspect(|(_, p)| tracing::info!("package `{name}` will be updated", name = p.name))
            .collect::<IndexMap<_, _>>();

        // if operator log and all packages are up to date at the latest checkpoint, then return
        if operator.checkpoint.is_some_and(|c| &c == checkpoint) && packages.is_empty() {
            return Ok(IndexMap::default());
        }

        // federated packages in other registries
        let mut federated_packages: IndexMap<Option<RegistryDomain>, Vec<&mut PackageInfo>> =
            IndexMap::with_capacity(packages.len());

        // loop and fetch logs
        let has_auth_token = self.api.auth_token().is_some();
        loop {
            let response = match self
                .api
                .fetch_logs(
                    registry_domain,
                    FetchLogsRequest {
                        log_length: checkpoint.log_length,
                        operator: operator
                            .head_fetch_token
                            .as_ref()
                            .map(|t| Cow::Borrowed(t.as_str())),
                        limit: None,
                        // last known fetch token for each package log ID
                        packages: Cow::Owned(
                            packages
                                .iter()
                                .map(|(id, p)| (id.clone(), p.head_fetch_token.clone()))
                                .collect::<IndexMap<_, _>>(),
                        ),
                    },
                )
                .await
                .inspect(|res| {
                    for warning in res.warnings.iter() {
                        tracing::warn!("Fetch warning from registry: {}", warning.message);
                    }
                }) {
                Ok(res) => Ok(res),
                Err(err) => match &err {
                    api::ClientError::Fetch(FetchError::LogNotFound(log_id))
                    | api::ClientError::Package(PackageError::LogNotFound(log_id)) => {
                        if let Some(name) = packages.get(log_id).map(|p| p.name.clone()) {
                            Err(ClientError::PackageDoesNotExist {
                                name,
                                has_auth_token,
                            })
                        } else {
                            Err(ClientError::Api(err))
                        }
                    }

                    api::ClientError::LogNotFoundWithHint(log_id, hint)
                        if self.disable_interactive =>
                    {
                        let name = packages.get(log_id).unwrap().name.clone();

                        match hint.to_str().ok().map(|s| s.split_once('=')) {
                            Some(Some((namespace, registry))) if packages.contains_key(log_id) => {
                                Err(ClientError::PackageDoesNotExistWithHintHeader {
                                    name,
                                    has_auth_token,
                                    hint_namespace: namespace.to_string(),
                                    hint_registry: registry.to_string(),
                                })
                            }
                            _ => Err(ClientError::PackageDoesNotExist {
                                name,
                                has_auth_token,
                            }),
                        }
                    }

                    #[cfg(feature = "cli-interactive")]
                    api::ClientError::LogNotFoundWithHint(log_id, hint) => {
                        match hint.to_str().ok().map(|s| s.split_once('=')) {
                            Some(Some((namespace, registry)))
                                if !self.ignore_federation_hints
                                    && packages.contains_key(log_id) =>
                            {
                                use dialoguer::{theme::ColorfulTheme, Confirm};

                                let package_name = &packages.get(log_id).unwrap().name;

                                if self.auto_accept_federation_hints
                                    || Confirm::with_theme(&ColorfulTheme::default())
                                        .with_prompt(format!(
"Package `{package_name}` is not in `{current_registry}` registry.
Registry recommends using `{registry}` registry for packages in `{namespace}` namespace.
Accept recommendation y/N\n",
current_registry = registry_domain.map(|d| d.as_str()).unwrap_or(&self.url().safe_label()),
))
                                        .default(true)
                                        .interact()
                                        .unwrap()
                                {
                                    let federated_registry_domain =
                                        Some(RegistryDomain::from_str(registry)?);
                                    self.store_namespace(
                                        namespace.to_string(),
                                        federated_registry_domain.clone().unwrap(),
                                    )
                                    .await?;

                                    // filter packages with namespace in other registry
                                    packages = packages
                                        .into_iter()
                                        .filter_map(|(log_id, package_info)| {
                                            if package_info.name.namespace() == namespace {
                                                if let Some(package_set) = federated_packages
                                                    .get_mut(&federated_registry_domain)
                                                {
                                                    package_set.push(package_info);
                                                } else {
                                                    federated_packages.insert(
                                                        federated_registry_domain.clone(),
                                                        vec![package_info],
                                                    );
                                                }

                                                None
                                            } else {
                                                Some((log_id, package_info))
                                            }
                                        })
                                        .collect();

                                    // continue fetching logs from this registry
                                    continue;
                                } else {
                                    Err(ClientError::PackageDoesNotExist {
                                        name: package_name.clone(),
                                        has_auth_token,
                                    })
                                }
                            }
                            _ => {
                                if let Some(name) = packages.get(log_id).map(|p| p.name.clone()) {
                                    Err(ClientError::PackageDoesNotExist {
                                        name,
                                        has_auth_token,
                                    })
                                } else {
                                    Err(ClientError::Api(err))
                                }
                            }
                        }
                    }
                    _ => Err(ClientError::Api(err)),
                },
            }?;

            for record in response.operator {
                let proto_envelope: PublishedProtoEnvelope<operator::OperatorRecord> =
                    record.envelope.try_into()?;

                // skip over records that has already seen
                if operator.head_registry_index.is_none()
                    || proto_envelope.registry_index > operator.head_registry_index.unwrap()
                {
                    operator.state = operator
                        .state
                        .validate(&proto_envelope.envelope)
                        .map_err(|inner| ClientError::OperatorValidationFailed { inner })?;
                    operator.head_registry_index = Some(proto_envelope.registry_index);
                    operator.head_fetch_token = Some(record.fetch_token);
                }
            }

            for (log_id, records) in response.packages {
                let package = packages.get_mut(&log_id).ok_or_else(|| {
                    anyhow!("received records for unknown package log `{log_id}`")
                })?;

                for record in records {
                    let proto_envelope: PublishedProtoEnvelope<package::PackageRecord> =
                        record.envelope.try_into()?;

                    // skip over records that has already seen
                    if package.head_registry_index.is_none()
                        || proto_envelope.registry_index > package.head_registry_index.unwrap()
                    {
                        let state = std::mem::take(&mut package.state);
                        package.state =
                            state.validate(&proto_envelope.envelope).map_err(|inner| {
                                ClientError::PackageValidationFailed {
                                    name: package.name.clone(),
                                    inner,
                                }
                            })?;
                        package.head_registry_index = Some(proto_envelope.registry_index);
                        package.head_fetch_token = Some(record.fetch_token);
                    }
                }

                // At this point, the package log should not be empty
                if package.state.head().is_none() {
                    return Err(ClientError::PackageLogEmpty {
                        name: package.name.clone(),
                    });
                }
            }

            if !response.more {
                break;
            }
        }

        // verify checkpoint signature
        TimestampedCheckpoint::verify(
            operator
                .state
                .public_key(ts_checkpoint.to_owned().to_owned().key_id())
                .ok_or(ClientError::InvalidCheckpointKeyId {
                    key_id: ts_checkpoint.key_id().clone(),
                })?,
            &ts_checkpoint.as_ref().encode(),
            ts_checkpoint.signature(),
        )
        .or(Err(ClientError::InvalidCheckpointSignature))?;

        // Prove inclusion for the current log heads
        let mut leaf_indices = Vec::with_capacity(packages.len() + 1 /* for operator */);
        let mut leafs = Vec::with_capacity(leaf_indices.len());

        // operator record inclusion
        if let Some(index) = operator.head_registry_index {
            leaf_indices.push(index);
            leafs.push(LogLeaf {
                log_id: LogId::operator_log::<Sha256>(),
                record_id: operator.state.head().as_ref().unwrap().digest.clone(),
            });
        } else {
            return Err(ClientError::NoOperatorRecords);
        }

        // package records inclusion
        for (log_id, package) in &packages {
            if let Some(index) = package.head_registry_index {
                leaf_indices.push(index);
                leafs.push(LogLeaf {
                    log_id: log_id.clone(),
                    record_id: package.state.head().as_ref().unwrap().digest.clone(),
                });
            } else {
                return Err(ClientError::PackageLogEmpty {
                    name: package.name.clone(),
                });
            }
        }

        if !leafs.is_empty() {
            self.api
                .prove_inclusion(
                    registry_domain,
                    InclusionRequest {
                        log_length: checkpoint.log_length,
                        leafs: leaf_indices,
                    },
                    checkpoint,
                    &leafs,
                )
                .await?;
        }

        if let Some(from) = self.registry.load_checkpoint(registry_domain).await? {
            let from_log_length = from.as_ref().checkpoint.log_length;
            let to_log_length = ts_checkpoint.as_ref().checkpoint.log_length;

            match from_log_length.cmp(&to_log_length) {
                Ordering::Greater => {
                    return Err(ClientError::CheckpointLogLengthRewind {
                        from: from_log_length,
                        to: to_log_length,
                    });
                }
                Ordering::Less => {
                    self.api
                        .prove_log_consistency(
                            registry_domain,
                            ConsistencyRequest {
                                from: from_log_length,
                                to: to_log_length,
                            },
                            Cow::Borrowed(&from.as_ref().checkpoint.log_root),
                            Cow::Borrowed(&ts_checkpoint.as_ref().checkpoint.log_root),
                        )
                        .await?
                }
                Ordering::Equal => {
                    if from.as_ref().checkpoint.log_root
                        != ts_checkpoint.as_ref().checkpoint.log_root
                        || from.as_ref().checkpoint.map_root
                            != ts_checkpoint.as_ref().checkpoint.map_root
                    {
                        return Err(ClientError::CheckpointChangedLogRootOrMapRoot {
                            log_length: from_log_length,
                        });
                    }
                }
            }
        }

        operator.registry = registry_domain
            .cloned()
            .or_else(|| Some(self.url().registry_domain()));
        operator.checkpoint = Some(checkpoint.clone()); // updated to this checkpoint
        self.registry
            .store_operator(registry_domain, operator)
            .await?;

        for package in packages.values_mut() {
            package.registry = registry_domain
                .cloned()
                .or_else(|| Some(self.url().registry_domain()));
            package.checkpoint = Some(checkpoint.clone()); // updated to this checkpoint
            self.registry
                .store_package(registry_domain, package)
                .await?;
        }

        self.registry
            .store_checkpoint(registry_domain, &ts_checkpoint)
            .await?;

        // return packages to be retrieved from other registries
        Ok(federated_packages)
    }

    /// Update checkpoint for list of packages
    async fn update_checkpoints<'a>(
        &self,
        packages: impl IntoIterator<Item = &mut PackageInfo>,
    ) -> Result<(), ClientError> {
        // first collect the packages that we already have namespace mappings for
        let mut federated_packages: IndexMap<Option<RegistryDomain>, Vec<&mut PackageInfo>> =
            IndexMap::new();
        for package in packages.into_iter() {
            let registry_domain = self.get_warg_registry(package.name.namespace()).await?;
            if let Some(package_set) = federated_packages.get_mut(&registry_domain) {
                package_set.push(package);
            } else {
                federated_packages.insert(registry_domain, vec![package]);
            }
        }

        while let Some((registry_domain, packages)) = federated_packages.pop() {
            for (registry_domain, packages) in self
                .update_packages_and_return_federated_packages(registry_domain.as_ref(), packages)
                .await?
                .into_iter()
            {
                if let Some(package_set) = federated_packages.get_mut(&registry_domain) {
                    package_set.extend(packages);
                } else {
                    federated_packages.insert(registry_domain, packages);
                }
            }
        }

        Ok(())
    }

    /// Fetches package logs without checking local storage first.
    pub async fn fetch_packages(
        &self,
        names: impl IntoIterator<Item = &PackageName>,
    ) -> Result<Vec<PackageInfo>, ClientError> {
        let mut packages: Vec<PackageInfo> = names
            .into_iter()
            .map(|name| PackageInfo::new(name.clone()))
            .collect();
        self.update_checkpoints(packages.iter_mut()).await?;
        Ok(packages)
    }

    /// Fetches the `PackageInfo` without checking local storage first.
    pub async fn fetch_package(&self, name: &PackageName) -> Result<PackageInfo, ClientError> {
        let mut info = PackageInfo::new(name.clone());
        self.update_checkpoints([&mut info]).await?;
        Ok(info)
    }

    /// Retrieves the `PackageInfo` from local storage, if present, otherwise fetches from the
    /// registry.
    pub async fn package(&self, name: &PackageName) -> Result<PackageInfo, ClientError> {
        let registry_domain = self.get_warg_registry(name.namespace()).await?;
        match self
            .registry
            .load_package(registry_domain.as_ref(), name)
            .await?
        {
            Some(mut info) => {
                tracing::info!("log for package `{name}` already exists in storage");
                if info.registry.is_none() {
                    info.registry = registry_domain
                        .clone()
                        .or_else(|| Some(self.url().registry_domain()));
                }
                Ok(info)
            }
            None => {
                let mut info = PackageInfo::new(name.clone());
                self.update_checkpoints([&mut info]).await?;
                Ok(info)
            }
        }
    }

    async fn get_package_record(
        &self,
        registry_domain: Option<&RegistryDomain>,
        package: &PackageName,
        log_id: &LogId,
        record_id: &RecordId,
    ) -> ClientResult<PackageRecord> {
        let record = self
            .api
            .get_package_record(registry_domain, log_id, record_id)
            .await
            .map_err(|e| match e {
                api::ClientError::Package(PackageError::Rejection(reason)) => {
                    ClientError::PublishRejected {
                        name: package.clone(),
                        reason,
                        record_id: record_id.clone(),
                    }
                }
                e => {
                    ClientError::translate_log_not_found(e, self.api.auth_token().is_some(), |id| {
                        if id == log_id {
                            Some(package.clone())
                        } else {
                            None
                        }
                    })
                }
            })?;
        Ok(record)
    }

    /// Downloads the content for the specified digest into client storage.
    ///
    /// If the content already exists in client storage, the existing path
    /// is returned.
    async fn download_content(
        &self,
        registry_domain: Option<&RegistryDomain>,
        digest: &AnyHash,
    ) -> Result<PathBuf, ClientError> {
        match self.content.content_location(digest) {
            Some(path) => {
                tracing::info!("content for digest `{digest}` already exists in storage");
                Ok(path)
            }
            None => {
                self.content
                    .store_content(
                        Box::pin(self.api.download_content(registry_domain, digest).await?),
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

    /// Downloads the content for the specified digest as a stream.
    ///
    /// If the content already exists in client storage, it is read from the client storage.
    ///
    /// The download is not stored in client storage.
    async fn download_content_stream(
        &self,
        registry_domain: Option<&RegistryDomain>,
        digest: &AnyHash,
    ) -> Result<impl Stream<Item = Result<Bytes>>, ClientError> {
        match self.content.content_location(digest) {
            Some(path) => {
                tracing::info!("content for digest `{digest}` already exists in storage");
                let file = tokio::fs::File::open(path)
                    .await
                    .map_err(ClientError::IoError)?;
                Ok(ReaderStream::new(file).map_err(Into::into).boxed())
            }
            None => Ok(Box::pin(
                self.api.download_content(registry_domain, digest).await?,
            )),
        }
    }
}
/// A Warg registry client that uses the local file system to store
/// package logs and content.
pub type FileSystemClient =
    Client<FileSystemRegistryStorage, FileSystemContentStorage, FileSystemNamespaceMapStorage>;

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
    /// If the URL is `None`, the home registry URL is used; if there is no home registry
    /// URL, an error is returned.
    ///
    /// If a lock cannot be acquired for a storage directory, then
    /// `NewClientResult::Blocked` is returned with the path to the
    /// directory that could not be locked.
    pub fn try_new_with_config(
        url: Option<&str>,
        config: &Config,
        mut auth_token: Option<Secret<String>>,
    ) -> Result<StorageLockResult<Self>, ClientError> {
        let StoragePaths {
            registry_url: url,
            registries_dir,
            content_dir,
            namespace_map_path,
        } = config.storage_paths_for_url(url)?;

        let (packages, content, namespace_map) = match (
            FileSystemRegistryStorage::try_lock(registries_dir.clone())?,
            FileSystemContentStorage::try_lock(content_dir.clone())?,
            FileSystemNamespaceMapStorage::new(namespace_map_path.clone()),
        ) {
            (Some(packages), Some(content), namespace_map) => (packages, content, namespace_map),
            (None, _, _) => return Ok(StorageLockResult::NotAcquired(registries_dir)),
            (_, None, _) => return Ok(StorageLockResult::NotAcquired(content_dir)),
        };

        let disable_interactive =
            cfg!(not(feature = "cli-interactive")) || config.disable_interactive;

        #[cfg(feature = "keyring")]
        if auth_token.is_none() && config.keyring_auth {
            auth_token = crate::keyring::Keyring::from_config(config)?.get_auth_token(&url)?
        }

        Ok(StorageLockResult::Acquired(Self::new(
            url.into_url(),
            packages,
            content,
            namespace_map,
            auth_token,
            config.ignore_federation_hints,
            config.auto_accept_federation_hints,
            disable_interactive,
        )?))
    }

    /// Attempts to create a client for the given registry URL.
    ///
    /// If the URL is `None`, the home registry URL is used; if there is no home registry
    /// URL, an error is returned.
    ///
    /// If a lock cannot be acquired for a storage directory, then
    /// `NewClientResult::Blocked` is returned with the path to the
    /// directory that could not be locked.
    ///
    /// Same as calling `try_new_with_config` with
    /// `Config::from_default_file()?.unwrap_or_default()`.
    pub fn try_new_with_default_config(
        url: Option<&str>,
    ) -> Result<StorageLockResult<Self>, ClientError> {
        Self::try_new_with_config(url, &Config::from_default_file()?.unwrap_or_default(), None)
    }

    /// Creates a client for the given registry URL.
    ///
    /// If the URL is `None`, the home registry URL is used; if there is no home registry
    /// URL, an error is returned.
    ///
    /// This method blocks if storage locks cannot be acquired.
    pub fn new_with_config(
        url: Option<&str>,
        config: &Config,
        mut auth_token: Option<Secret<String>>,
    ) -> Result<Self, ClientError> {
        let StoragePaths {
            registry_url,
            registries_dir,
            content_dir,
            namespace_map_path,
        } = config.storage_paths_for_url(url)?;

        let disable_interactive =
            cfg!(not(feature = "cli-interactive")) || config.disable_interactive;

        #[cfg(feature = "keyring")]
        if auth_token.is_none() && config.keyring_auth {
            auth_token =
                crate::keyring::Keyring::from_config(config)?.get_auth_token(&registry_url)?
        }

        Self::new(
            registry_url.into_url(),
            FileSystemRegistryStorage::lock(registries_dir)?,
            FileSystemContentStorage::lock(content_dir)?,
            FileSystemNamespaceMapStorage::new(namespace_map_path),
            auth_token,
            config.ignore_federation_hints,
            config.auto_accept_federation_hints,
            disable_interactive,
        )
    }

    /// Creates a client for the given registry URL.
    ///
    /// If the URL is `None`, the home registry URL is used; if there is no home registry
    /// URL, an error is returned.
    ///
    /// This method blocks if storage locks cannot be acquired.
    ///
    /// Same as calling `new_with_config` with
    /// `Config::from_default_file()?.unwrap_or_default()`.
    pub fn new_with_default_config(url: Option<&str>) -> Result<Self, ClientError> {
        Self::new_with_config(url, &Config::from_default_file()?.unwrap_or_default(), None)
    }
}

/// Represents information about a downloaded package.
#[derive(Debug, Clone)]
pub struct PackageDownload {
    /// The package version that was downloaded.
    pub version: Version,
    /// The digest of the package contents.
    pub digest: AnyHash,
    /// The path to the downloaded package contents.
    pub path: PathBuf,
}

/// Represents information about a downloaded package.
pub struct PackageDownloadInfo {
    /// The package version that was downloaded.
    pub version: Version,
    /// The digest of the package contents.
    pub digest: AnyHash,
}

/// Represents an error returned by Warg registry clients.
#[derive(Debug, Error)]
pub enum ClientError {
    /// No home registry registry server URL is configured.
    #[error("no home registry registry server URL is configured")]
    NoHomeRegistryUrl,

    /// Reset registry local state.
    #[error("reset registry state failed")]
    ResettingRegistryLocalStateFailed,

    /// Clearing content local cache.
    #[error("clear content cache failed")]
    ClearContentCacheFailed,

    /// Unauthorized rejection
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// Checkpoint signature failed verification
    #[error("invalid checkpoint signature")]
    InvalidCheckpointSignature,

    /// Checkpoint signature failed verification
    #[error("invalid checkpoint key ID `{key_id}`")]
    InvalidCheckpointKeyId {
        /// The signature key ID.
        key_id: signing::KeyID,
    },

    /// The server did not provide operator records.
    #[error("the server did not provide any operator records")]
    NoOperatorRecords,

    /// The operator failed validation.
    #[error("operator failed validation: {inner}")]
    OperatorValidationFailed {
        /// The validation error.
        inner: operator::ValidationError,
    },

    /// The package already exists and cannot be initialized.
    #[error("package `{name}` already exists and cannot be initialized")]
    CannotInitializePackage {
        /// The package name that already exists.
        name: PackageName,
        /// The record identifier for the init record.
        init_record_id: Option<RecordId>,
    },

    /// The package must be initialized before publishing.
    #[error("package `{name}` must be initialized before publishing")]
    MustInitializePackage {
        /// The name of the package that must be initialized.
        name: PackageName,
        /// Client has authentication credentials.
        has_auth_token: bool,
    },

    /// There is no publish operation in progress.
    #[error("there is no publish operation in progress")]
    NotPublishing,

    /// The package has no records to publish.
    #[error("package `{name}` has no records to publish")]
    NothingToPublish {
        /// The package that has no publish operations.
        name: PackageName,
    },

    /// The package does not exist.
    #[error("package `{name}` does not exist")]
    PackageDoesNotExist {
        /// The missing package.
        name: PackageName,
        /// Client has authentication credentials.
        has_auth_token: bool,
    },

    /// The package does not exist with hint header.
    #[error("package `{name}` does not exist but the registry suggests checking registry `{hint_registry}` for packages in namespace `{hint_namespace}`")]
    PackageDoesNotExistWithHintHeader {
        /// The missing package.
        name: PackageName,
        /// Client has authentication credentials.
        has_auth_token: bool,
        /// The hint namespace.
        hint_namespace: String,
        /// The hint registry.
        hint_registry: String,
    },

    /// The package version does not exist.
    #[error("version `{version}` of package `{name}` does not exist")]
    PackageVersionDoesNotExist {
        /// The missing version of the package.
        version: Version,
        /// The package with the missing version.
        name: PackageName,
    },

    /// The package version requirement does not exist.
    #[error("version that satisfies requirement `{version}` was not found for package `{name}`")]
    PackageVersionRequirementDoesNotExist {
        /// The missing version requirement of the package.
        version: VersionReq,
        /// The package with the missing version.
        name: PackageName,
    },

    /// The package failed validation.
    #[error("package `{name}` failed validation: {inner}")]
    PackageValidationFailed {
        /// The package that failed validation.
        name: PackageName,
        /// The validation error.
        inner: package::ValidationError,
    },

    /// Content was not found during a publish operation.
    #[error("content with digest `{digest}` was not found in client storage")]
    ContentNotFound {
        /// The digest of the missing content.
        digest: AnyHash,
    },

    /// Content digest was different than expected.
    #[error("content with digest `{digest}` was not found expected `{expected}`")]
    IncorrectContent {
        /// The digest of the missing content.
        digest: AnyHash,
        /// The expected
        expected: AnyHash,
    },

    /// The package log is empty and cannot be validated.
    #[error("package log is empty and cannot be validated")]
    PackageLogEmpty {
        /// The package with an empty package log.
        name: PackageName,
    },

    /// A publish operation was rejected.
    #[error("the publishing of package `{name}` was rejected due to: {reason}")]
    PublishRejected {
        /// The package that was rejected.
        name: PackageName,
        /// The record identifier for the record that was rejected.
        record_id: RecordId,
        /// The reason it was rejected.
        reason: String,
    },

    /// A publish operation was rejected due to conflicting pending publish.
    #[error("the publishing of package `{name}` was rejected due to conflicting pending publish of record `{pending_record_id}`")]
    ConflictPendingPublish {
        /// The package that was rejected.
        name: PackageName,
        /// The record identifier for the record that was rejected.
        record_id: RecordId,
        /// The record identifier for the pending publish record.
        pending_record_id: RecordId,
    },

    /// The package is still missing content.
    #[error("the package is still missing content after all content was uploaded")]
    PackageMissingContent,

    /// The registry provided a latest checkpoint with a log length less than a previously provided
    /// checkpoint log length.
    #[error("registry rewinded checkpoints; latest checkpoint log length `{to}` is less than previously received checkpoint log length `{from}`")]
    CheckpointLogLengthRewind {
        /// The previously received checkpoint log length.
        from: RegistryLen,
        /// The latest checkpoint log length.
        to: RegistryLen,
    },

    /// The registry provided a checkpoint with a different `log_root` and
    /// `map_root` than a previously provided checkpoint.
    #[error("registry provided a new checkpoint with the same log length `{log_length}` as previously fetched but different log root or map root")]
    CheckpointChangedLogRootOrMapRoot {
        /// The checkpoint log length.
        log_length: RegistryLen,
    },

    /// An error occurred while accessing the keyring.
    #[error(transparent)]
    Keyring(#[from] crate::keyring::KeyringError),

    /// An error occurred during an API operation.
    #[error(transparent)]
    Api(#[from] api::ClientError),

    /// An error occurred while performing a client operation.
    #[error("{0:?}")]
    Other(#[from] anyhow::Error),

    /// An error occurred while performing a IO.
    #[error("error: {0:?}")]
    IoError(#[from] std::io::Error),
}

impl ClientError {
    fn translate_log_not_found(
        e: api::ClientError,
        has_auth_token: bool,
        lookup: impl Fn(&LogId) -> Option<PackageName>,
    ) -> Self {
        match &e {
            api::ClientError::Fetch(FetchError::LogNotFound(id))
            | api::ClientError::Package(PackageError::LogNotFound(id)) => {
                if let Some(name) = lookup(id) {
                    return Self::PackageDoesNotExist {
                        name,
                        has_auth_token,
                    };
                }
            }
            _ => {}
        }

        Self::Api(e)
    }
}

/// Represents the result of a client operation.
pub type ClientResult<T> = Result<T, ClientError>;
