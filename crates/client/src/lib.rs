pub mod api;
pub mod file_storage;
pub mod storage;

use std::sync::Arc;

use anyhow::Context;
use futures_util::StreamExt;
use indexmap::IndexMap;
use thiserror::Error;
use warg_crypto::{
    hash::{DynHash, Hash, HashAlgorithm, Sha256},
    signing,
};
use warg_protocol::{
    package::{self, ValidationError},
    registry::{LogId, LogLeaf, MapCheckpoint},
    ProtoEnvelope, SerdeEnvelope, Version,
};

pub use file_storage::FileSystemStorage;
pub use storage::{
    ClientStorage, ExpectedContent, NewContent, PackageEntryInfo, PublishInfo, RegistryInfo,
};
use warg_server::services::core::{ContentSource, ContentSourceKind};

pub struct Client {
    storage: Box<dyn storage::ClientStorage>,
}

impl Client {
    pub fn new(storage: Box<dyn storage::ClientStorage>) -> Self {
        Self { storage }
    }

    async fn registry_info(&self) -> Result<RegistryInfo, ClientError> {
        match self.storage.load_registry_info().await? {
            Some(reg_info) => Ok(reg_info),
            None => Err(ClientError::RegistryNotSet),
        }
    }

    pub fn storage(&mut self) -> &mut Box<dyn storage::ClientStorage> {
        &mut self.storage
    }

    pub async fn start_publish_init(
        &mut self,
        package: String,
        author: signing::PublicKey,
    ) -> Result<(), ClientError> {
        if self.storage.load_publish_info().await?.is_some() {
            return Err(ClientError::AlreadyPublishing);
        }

        let package_state = self.storage.load_package_state(&package).await?;
        if package_state.head().is_some() {
            return Err(ClientError::InitAlreadyExistingPackage);
        }

        let mut info = PublishInfo::new(package, None);
        info.push_init(HashAlgorithm::Sha256, author);
        self.storage.store_publish_info(&info).await?;
        Ok(())
    }

    pub async fn start_publish(&mut self, package: String) -> Result<(), ClientError> {
        if self.storage.load_publish_info().await?.is_some() {
            return Err(ClientError::AlreadyPublishing);
        }

        let package_state = self.storage.load_package_state(&package).await?;
        let Some(package_head) = package_state.head().as_ref().map(|h| h.digest.clone()) else {
            return Err(ClientError::PublishToNonExistingPackage);
        };

        let info = PublishInfo::new(package, Some(package_head));
        self.storage.store_publish_info(&info).await?;
        Ok(())
    }

    pub async fn cancel_publish(&mut self) -> Result<(), ClientError> {
        self.storage
            .clear_publish_info()
            .await
            .with_context(|| "Error clearing publish contents")?;
        Ok(())
    }

    pub async fn queue_release(
        &mut self,
        version: Version,
        digest: DynHash,
    ) -> Result<(), ClientError> {
        if let Some(mut pub_info) = self.storage.load_publish_info().await? {
            pub_info.push_release(version, digest);
            self.storage.store_publish_info(&pub_info).await?;
            Ok(())
        } else {
            Err(ClientError::NotPublishing)
        }
    }

    pub async fn submit_publish(
        &mut self,
        signing_key: &signing::PrivateKey,
    ) -> Result<(), ClientError> {
        let reg_info = self.registry_info().await?;
        let client = api::Client::new(reg_info.url.to_owned());

        if let Some(pub_info) = self.storage.load_publish_info().await? {
            let (name, contents, record) = pub_info.finalize();
            let record =
                ProtoEnvelope::signed_contents(signing_key, record).map_err(anyhow::Error::new)?;

            let mut content_sources = Vec::new();
            if !contents.is_empty() {
                for digest in contents {
                    let content = self.storage.get_content(&digest).await?;
                    let content = content.ok_or_else(|| ClientError::NeededContentNotFound {
                        digest: digest.clone(),
                    })?;
                    let url = client.upload_content(content).await?;
                    content_sources.push(ContentSource {
                        content_digest: digest,
                        kind: ContentSourceKind::HttpAnonymous { url },
                    })
                }
            }
            let response = client
                .publish(&name, Arc::new(record), content_sources)
                .await?;

            self.install(name).await?;

            self.update_to(response.checkpoint.as_ref()).await?;
            Ok(())
        } else {
            Err(ClientError::NotPublishing)
        }
    }

    pub async fn install(&mut self, package: String) -> Result<(), ClientError> {
        let mut reg_info = self.registry_info().await?;
        let client = api::Client::new(reg_info.url.to_owned());
        let checkpoint = match reg_info.checkpoint {
            Some(checkpoint) => checkpoint,
            None => {
                let checkpoint = client.latest_checkpoint().await?;
                reg_info.checkpoint = Some(checkpoint.clone());
                self.storage.store_registry_info(&reg_info).await?;
                checkpoint
            }
        };

        self.update_packages(&client, &checkpoint, vec![package])
            .await?;
        Ok(())
    }

    pub async fn update(&mut self) -> Result<(), ClientError> {
        let mut reg_info = self.registry_info().await?;
        let client = api::Client::new(reg_info.url.to_owned());
        let checkpoint = client.latest_checkpoint().await?;
        let packages = self.storage.list_all_packages().await?;
        self.update_packages(&client, &checkpoint, packages).await?;
        reg_info.checkpoint = Some(checkpoint);
        self.storage.store_registry_info(&reg_info).await?;
        Ok(())
    }

    pub async fn update_to(
        &mut self,
        checkpoint: &SerdeEnvelope<MapCheckpoint>,
    ) -> Result<(), ClientError> {
        let reg_info = self.registry_info().await?;
        let client = api::Client::new(reg_info.url.to_owned());
        let packages = self.storage.list_all_packages().await?;
        self.update_packages(&client, checkpoint, packages).await
    }

    async fn update_packages(
        &mut self,
        client: &api::Client,
        checkpoint: &SerdeEnvelope<MapCheckpoint>,
        packages: Vec<String>,
    ) -> Result<(), ClientError> {
        let root: Hash<Sha256> = Hash::of(checkpoint.as_ref());
        let root: DynHash = root.into();

        let mut validators = Vec::new();
        let mut heads = Vec::new();
        for name in packages {
            let state = self.storage.load_package_state(&name).await?;
            let head = state.head().as_ref().map(|head| head.digest.clone());
            validators.push((name.clone(), state));
            heads.push((name, head));
        }

        let packages = IndexMap::from_iter(heads.into_iter());
        let mut response = client
            .fetch_logs(api::FetchRequest {
                root,
                operator: None,
                packages,
            })
            .await?;

        let mut heads = Vec::new();
        for (name, state) in validators.iter_mut() {
            let new_records = response.packages.remove(name).ok_or_else(|| {
                ClientError::RequestedPackageOmitted {
                    package: name.to_owned(),
                }
            })?;

            for envelope in new_records {
                let envelope: ProtoEnvelope<package::PackageRecord> = envelope.try_into()?;
                let needed_content = state.validate(&envelope).map_err(|inner| {
                    ClientError::PackageValidationError {
                        package: name.to_owned(),
                        inner,
                    }
                })?;
                for digest in needed_content {
                    self.download_content(client, digest).await?;
                }
            }

            if let Some(head) = state.head() {
                let log_id = LogId::package_log::<Sha256>(name);
                let record_id = head.digest.clone();
                let leaf = LogLeaf { log_id, record_id };
                heads.push(leaf);
            } else {
                return Err(ClientError::PackageLogEmpty);
            }
        }

        client.prove_inclusion(checkpoint.as_ref(), heads).await?;

        for (name, state) in validators.iter() {
            self.storage.store_package_state(name, state).await?;
        }

        Ok(())
    }

    async fn download_content(
        &mut self,
        client: &api::Client,
        digest: DynHash,
    ) -> Result<(), ClientError> {
        let mut stream = client.download_content(&digest).await?;
        let mut expected_content = self.storage.store_content(digest).await?;
        while let Some(bytes) = stream
            .next()
            .await
            .transpose()
            .map_err(anyhow::Error::new)?
        {
            expected_content.write_all(bytes.as_ref()).await?;
        }
        expected_content.finalize().await?;
        Ok(())
    }

    pub async fn get_latest_version(&self, package: &String) -> Result<DynHash, ClientError> {
        let state = self.storage.load_package_state(package).await?;
        let release = state
            .find_latest_release(&Default::default())
            .with_context(|| format!("No release found for package {package}"))?;
        let content_digest = release
            .content()
            .with_context(|| format!("No content for release {package} {}", release.version))?;
        Ok(content_digest.to_owned())
    }
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Must have a registry set.")]
    RegistryNotSet,

    #[error("Already publishing, cannot begin a new publish.")]
    AlreadyPublishing,

    #[error("Not currently publishing.")]
    NotPublishing,

    #[error("Cannot anitialize already existing package.")]
    InitAlreadyExistingPackage,

    #[error("Cannot publish to package that does not exist.")]
    PublishToNonExistingPackage,

    #[error("Needed content {digest}, but was not found.")]
    NeededContentNotFound { digest: DynHash },

    #[error("Registry did not respond with the requested package \"{package}\".")]
    RequestedPackageOmitted { package: String },

    #[error("Package log \"{package}\" was invalid.")]
    PackageValidationError {
        package: String,
        inner: ValidationError,
    },

    #[error("Package claimed to be empty, cannot validate whether true.")]
    PackageLogEmpty,

    #[error("Error while installing package.")]
    OtherError(#[from] anyhow::Error),
}
