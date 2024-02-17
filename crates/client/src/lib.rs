//! A client library for Warg component registries.

#![deny(missing_docs)]
use crate::storage::PackageInfo;
use anyhow::{anyhow, Context, Result};
use reqwest::header::HeaderValue;
use reqwest::{Body, IntoUrl};
use semver::{Version, VersionReq};
use std::cmp::Ordering;
use std::fs;
use std::str::FromStr;
use std::{borrow::Cow, collections::HashMap, path::PathBuf, time::Duration};
use storage::{
    ContentStorage, FileSystemContentStorage, FileSystemNamespaceMapStorage,
    FileSystemRegistryStorage, NamespaceMapStorage, PublishInfo, RegistryStorage,
};
use thiserror::Error;
use warg_api::v1::{
    fetch::{FetchError, FetchLogsRequest, FetchLogsResponse},
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
    PublishedProtoEnvelope, SerdeEnvelope,
};
use wasm_compose::graph::{CompositionGraph, EncodeOptions, ExportIndex, InstanceId};

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
}

impl<R: RegistryStorage, C: ContentStorage, N: NamespaceMapStorage> Client<R, C, N> {
    /// Creates a new client for the given URL, registry storage, and
    /// content storage.
    pub fn new(url: impl IntoUrl, registry: R, content: C, namespace_map: N) -> ClientResult<Self> {
        Ok(Self {
            registry,
            content,
            namespace_map,
            api: api::Client::new(url)?,
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

    /// Stores namespace mapping in local storage
    pub async fn store_namespace(&self, namespace: String, registry_domain: String) -> Result<()> {
        self.namespace_map
            .store_namespace(namespace, registry_domain)
            .await?;
        Ok(())
    }

    /// Gets the namespace map
    pub async fn reset_namespaces(&self) -> Result<()> {
        self.namespace_map.reset_namespaces().await?;
        Ok(())
    }

    /// Reset client storage for the registry.
    pub async fn reset_registry(&self, all_registries: bool) -> ClientResult<()> {
        tracing::info!("resetting registry local state");
        self.registry
            .reset(all_registries)
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

    /// Check operator log for namespace mapping
    pub async fn refresh_namespace(
        &mut self,
        auth_token: &Option<String>,
        namespace: &str,
    ) -> ClientResult<()> {
        self.update_checkpoint(auth_token, &self.api.latest_checkpoint().await?, vec![])
            .await?;
        let operator = self.registry().load_operator(&None).await?;
        let operator_log_maps_namespace = if let Some(op) = operator {
            let namespace_state = op.state.namespace_state(namespace);
            if let Ok(Some(nm)) = namespace_state {
                match nm {
                    warg_protocol::operator::NamespaceState::Defined => true,
                    warg_protocol::operator::NamespaceState::Imported { registry } => {
                        self.api
                            .map_warg_header(Some(HeaderValue::from_str(registry).unwrap()));

                        true
                    }
                }
            } else {
                false
            }
        } else {
            false
        };
        if !operator_log_maps_namespace {
            let map = self.namespace_map().load_namespace_map().await?;
            if let Some(map) = map {
                let namespace = map.get(namespace);
                if let Some(nm) = namespace {
                    self.api
                        .map_warg_header(Some(HeaderValue::from_str(nm).unwrap()));
                } else {
                    self.api.map_warg_header(None);
                }
            }
        }
        Ok(())
    }

    /// Get namespace registry
    pub fn get_warg_header(&self) -> &Option<HeaderValue> {
        self.api.get_warg_header()
    }

    /// Locks component
    pub async fn lock_component(
        &self,
        auth_token: &Option<String>,
        info: &PackageInfo,
    ) -> ClientResult<Vec<u8>> {
        let mut builder = LockListBuilder::default();
        builder.build_list(auth_token, self, info).await?;
        let top = Import {
            name: format!("{}:{}", info.name.namespace(), info.name.name()),
            req: VersionReq::STAR,
            kind: ImportKind::Unlocked,
        };
        builder.lock_list.insert(top);
        let mut composer = CompositionGraph::new();
        let mut handled = HashMap::<String, InstanceId>::new();
        for package in builder.lock_list {
            let name = package.name.clone();
            let version = package.req;
            let id = PackageName::new(name)?;
            let info = self
                .registry()
                .load_package(self.get_warg_header(), &id)
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
    pub async fn bundle_component(
        &self,
        auth_token: &Option<String>,
        info: &PackageInfo,
    ) -> ClientResult<Vec<u8>> {
        let mut bundler = Bundler::new(self);
        let path = PathBuf::from("./locked.wasm");
        let locked = if !path.is_file() {
            self.lock_component(auth_token, info).await?
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
    pub async fn publish(
        &self,
        auth_token: &Option<String>,
        signing_key: &signing::PrivateKey,
    ) -> ClientResult<RecordId> {
        let info = self
            .registry
            .load_publish()
            .await?
            .ok_or(ClientError::NotPublishing)?;

        let res = self.publish_with_info(auth_token, signing_key, info).await;
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
        auth_token: &Option<String>,
        signing_key: &signing::PrivateKey,
        mut info: PublishInfo,
    ) -> ClientResult<RecordId> {
        if info.entries.is_empty() {
            return Err(ClientError::NothingToPublish {
                name: info.name.clone(),
            });
        }

        let initializing = info.initializing();

        tracing::info!(
            "publishing {new}package `{name}`",
            name = info.name,
            new = if initializing { "new " } else { "" }
        );
        tracing::debug!("entries: {:?}", info.entries);

        let mut package = self
            .registry
            .load_package(self.get_warg_header(), &info.name)
            .await?
            .unwrap_or_else(|| PackageInfo::new(info.name.clone()));

        // If we're not initializing the package and a head was not explicitly specified,
        // updated to the latest checkpoint to get the latest known head.
        if !initializing && info.head.is_none() {
            self.update_checkpoint(
                auth_token,
                &self.api.latest_checkpoint().await?,
                [&mut package],
            )
            .await?;

            info.head = package.state.head().as_ref().map(|h| h.digest.clone());
        }

        match (initializing, info.head.is_some()) {
            (true, true) => {
                return Err(ClientError::CannotInitializePackage { name: package.name })
            }
            (false, false) => {
                return Err(ClientError::MustInitializePackage { name: package.name })
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
                    package_name: Cow::Borrowed(&package.name),
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
                        name: package.clone(),
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

    /// Updates every package log in every client registry storage to the latest registry checkpoint.
    pub async fn update_all(&mut self, auth_token: &Option<String>) -> ClientResult<()> {
        let packages = self.registry.load_all_packages().await?;
        let checkpoints = self.api.latest_checkpoints(packages.keys()).await?;
        self.update_checkpoints(auth_token, checkpoints, packages)
            .await?;
        Ok(())
    }

    /// Updates every package log in client storage to the latest registry checkpoint.
    pub async fn update(&self, auth_token: &Option<String>) -> ClientResult<()> {
        tracing::info!("updating all packages to latest checkpoint");

        let mut updating = self.registry.load_packages().await?;

        self.update_checkpoint(
            auth_token,
            &self.api.latest_checkpoint().await?,
            &mut updating,
        )
        .await?;

        Ok(())
    }

    /// Inserts or updates the logs of the specified packages in client storage to
    /// the latest registry checkpoint.
    pub async fn upsert<'a, I>(
        &self,
        auth_token: &Option<String>,
        packages: I,
    ) -> Result<(), ClientError>
    where
        I: IntoIterator<Item = &'a PackageName>,
        I::IntoIter: ExactSizeIterator,
    {
        tracing::info!("updating specific packages to latest checkpoint");

        let packages = packages.into_iter();
        let mut updating = Vec::with_capacity(packages.len());
        for package in packages {
            updating.push(
                self.registry
                    .load_package(self.get_warg_header(), package)
                    .await?
                    .unwrap_or_else(|| PackageInfo::new(package.clone())),
            );
        }

        self.update_checkpoint(
            auth_token,
            &self.api.latest_checkpoint().await?,
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
        auth_token: &Option<String>,
        name: &PackageName,
        requirement: &VersionReq,
    ) -> Result<Option<PackageDownload>, ClientError> {
        tracing::info!("downloading package `{name}` with requirement `{requirement}`");
        let info = self.fetch_package(auth_token, name).await?;

        match info.state.find_latest_release(requirement) {
            Some(release) => {
                let digest = release
                    .content()
                    .context("invalid state: not yanked but missing content")?
                    .clone();
                let path = self.download_content(auth_token, &digest).await?;
                Ok(Some(PackageDownload {
                    version: release.version.clone(),
                    digest,
                    path,
                }))
            }
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
        auth_token: &Option<String>,
        package: &PackageName,
        version: &Version,
    ) -> Result<PackageDownload, ClientError> {
        tracing::info!("downloading version {version} of package `{package}`");
        let info = self.fetch_package(auth_token, package).await?;

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
            path: self.download_content(auth_token, digest).await?,
        })
    }

    /// Update checkpoint for list of packages
    async fn update_checkpoint<'a>(
        &self,
        auth_token: &Option<String>,
        ts_checkpoint: &SerdeEnvelope<TimestampedCheckpoint>,
        packages: impl IntoIterator<Item = &mut PackageInfo>,
    ) -> Result<(), ClientError> {
        let checkpoint = &ts_checkpoint.as_ref().checkpoint;
        tracing::info!(
            "updating to checkpoint log length `{}`",
            checkpoint.log_length
        );

        let mut operator = self
            .registry
            .load_operator(self.get_warg_header())
            .await?
            .unwrap_or_default();

        // Map package names to package logs that need to be updated
        let mut packages = packages
            .into_iter()
            .filter_map(|p| match &p.checkpoint {
                // Don't bother updating if the package is already at the specified checkpoint
                Some(c) if c == checkpoint => None,
                _ => Some((LogId::package_log::<Sha256>(&p.name), p)),
            })
            .inspect(|(_, p)| tracing::info!("package `{name}` will be updated", name = p.name))
            .collect::<HashMap<_, _>>();
        if packages.is_empty() {
            return Ok(());
        }

        let mut last_known = packages
            .iter()
            .map(|(id, p)| (id.clone(), p.head_fetch_token.clone()))
            .collect::<HashMap<_, _>>();

        loop {
            // let response: FetchLogsResponse = match self
            let response: FetchLogsResponse = self
                .api
                .fetch_logs(
                    FetchLogsRequest {
                        log_length: checkpoint.log_length,
                        operator: operator
                            .head_fetch_token
                            .as_ref()
                            .map(|t| Cow::Borrowed(t.as_str())),
                        limit: None,
                        packages: Cow::Borrowed(&last_known),
                    },
                    auth_token,
                )
                .await
                .map_err(|e| {
                    ClientError::translate_log_not_found(e, |id| {
                        packages.get(id).map(|p| p.name.clone())
                    })
                })?;

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

            // Update the last known record fetch token for each package log
            for (id, fetch_token) in last_known.iter_mut() {
                *fetch_token = packages[id].head_fetch_token.clone();
            }
        }

        // verify checkpoint signature
        TimestampedCheckpoint::verify(
            operator.state.public_key(ts_checkpoint.key_id()).ok_or(
                ClientError::InvalidCheckpointKeyId {
                    key_id: ts_checkpoint.key_id().clone(),
                },
            )?,
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
                    auth_token,
                    InclusionRequest {
                        log_length: checkpoint.log_length,
                        leafs: leaf_indices,
                    },
                    checkpoint,
                    &leafs,
                )
                .await?;
        }

        if let Some(from) = self
            .registry
            .load_checkpoint(self.get_warg_header())
            .await?
        {
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
                            auth_token,
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

        self.registry
            .store_operator(self.get_warg_header(), operator)
            .await?;

        for package in packages.values_mut() {
            package.checkpoint = Some(checkpoint.clone());
            self.registry
                .store_package(self.get_warg_header(), package)
                .await?;
        }

        self.registry
            .store_checkpoint(self.get_warg_header(), ts_checkpoint)
            .await?;

        Ok(())
    }

    /// Update client namespace
    pub fn map_warg_header(&mut self, namespace: &Option<HeaderValue>) {
        self.api.map_warg_header(namespace.clone());
    }

    async fn update_checkpoints<'a>(
        &mut self,
        auth_token: &Option<String>,
        ts_checkpoints: HashMap<std::string::String, SerdeEnvelope<TimestampedCheckpoint>>,
        mut packages: HashMap<String, Vec<PackageInfo>>,
    ) -> Result<(), ClientError> {
        for (name, ts_checkpoint) in ts_checkpoints {
            if self.url().safe_label() != name {
                self.map_warg_header(&Some(HeaderValue::from_str(&name).unwrap()));
            } else {
                self.map_warg_header(&None)
            }
            let mut packages = packages.get_mut(&name.clone());
            if let Some(pkgs) = &mut packages {
                self.update_checkpoint(auth_token, &ts_checkpoint, pkgs.as_mut_slice())
                    .await?;
            }
        }

        Ok(())
    }

    async fn fetch_package(
        &self,
        auth_token: &Option<String>,
        name: &PackageName,
    ) -> Result<PackageInfo, ClientError> {
        match self
            .registry
            .load_package(self.get_warg_header(), name)
            .await?
        {
            Some(info) => {
                tracing::info!("log for package `{name}` already exists in storage");
                Ok(info)
            }
            None => {
                let mut info = PackageInfo::new(name.clone());
                self.update_checkpoint(
                    auth_token,
                    &self.api.latest_checkpoint().await?,
                    [&mut info],
                )
                .await?;

                Ok(info)
            }
        }
    }

    async fn get_package_record(
        &self,
        package: &PackageName,
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
                        Some(package.clone())
                    } else {
                        None
                    }
                })
            })?;
        Ok(record)
    }

    /// Downloads the content for the specified digest into client storage.
    ///
    /// If the content already exists in client storage, the existing path
    /// is returned.
    pub async fn download_content(
        &self,
        auth_token: &Option<String>,
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
                        Box::pin(self.api.download_content(auth_token, digest).await?),
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

        Ok(StorageLockResult::Acquired(Self::new(
            url.into_url(),
            packages,
            content,
            namespace_map,
        )?))
    }

    /// Creates a client for the given registry URL.
    ///
    /// If the URL is `None`, the home registry URL is used; if there is no home registry
    /// URL, an error is returned.
    ///
    /// This method blocks if storage locks cannot be acquired.
    pub fn new_with_config(url: Option<&str>, config: &Config) -> Result<Self, ClientError> {
        let StoragePaths {
            registry_url,
            registries_dir,
            content_dir,
            namespace_map_path,
        } = config.storage_paths_for_url(url)?;
        Self::new(
            registry_url.into_url(),
            FileSystemRegistryStorage::lock(registries_dir)?,
            FileSystemContentStorage::lock(content_dir)?,
            FileSystemNamespaceMapStorage::new(namespace_map_path),
        )
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
    },

    /// The package must be initialized before publishing.
    #[error("package `{name}` must be initialized before publishing")]
    MustInitializePackage {
        /// The name of the package that must be initialized.
        name: PackageName,
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
    },

    /// The package does not exist with hint.
    #[error("package `{name}` does not exist")]
    PackageDoesNotExistWithHint {
        /// The missing package.
        name: PackageName,
        /// The registry hint
        hint: HeaderValue,
    },

    /// The package version does not exist.
    #[error("version `{version}` of package `{name}` does not exist")]
    PackageVersionDoesNotExist {
        /// The missing version of the package.
        version: Version,
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
        lookup: impl Fn(&LogId) -> Option<PackageName>,
    ) -> Self {
        match &e {
            api::ClientError::Fetch(FetchError::LogNotFound(id))
            | api::ClientError::Package(PackageError::LogNotFound(id)) => {
                if let Some(name) = lookup(id) {
                    return Self::PackageDoesNotExist { name };
                }
            }
            api::ClientError::LogNotFoundWithHint(log_id, hint) => {
                if let Some(name) = lookup(log_id) {
                    return Self::PackageDoesNotExistWithHint {
                        name,
                        hint: hint.clone(),
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
