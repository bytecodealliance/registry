use std::time::SystemTime;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use warg_crypto::{
    hash::{DynHash, HashAlgorithm},
    signing,
};
use warg_protocol::{
    package,
    registry::{MapCheckpoint, RecordId},
    SerdeEnvelope, Version,
};

#[async_trait]
pub trait ClientStorage {
    async fn load_registry_info(&self) -> Result<Option<RegistryInfo>>;

    async fn store_registry_info(&mut self, info: &RegistryInfo) -> Result<()>;

    async fn load_publish_info(&self) -> Result<Option<PublishInfo>>;

    async fn store_publish_info(&mut self, info: &PublishInfo) -> Result<()>;

    async fn clear_publish_info(&mut self) -> Result<()>;

    async fn list_all_packages(&self) -> Result<Vec<String>>;

    async fn load_package_state(&self, package: &String) -> Result<package::Validator>;

    async fn store_package_state(
        &mut self,
        package: &String,
        state: &package::Validator,
    ) -> Result<()>;

    async fn has_content(&self, digest: &DynHash) -> Result<bool>;

    async fn store_content<'s>(
        &'s mut self,
        digest: DynHash,
    ) -> Result<Box<dyn ExpectedContent + 's>>;

    async fn create_content<'s>(&'s mut self) -> Result<Box<dyn NewContent + 's>>;

    async fn get_content(&self, digest: &DynHash) -> Result<Option<Vec<u8>>>;
}

#[derive(Serialize, Deserialize)]
pub struct RegistryInfo {
    pub url: String,
    pub checkpoint: Option<SerdeEnvelope<MapCheckpoint>>,
}

#[derive(Serialize, Deserialize)]
pub struct PublishInfo {
    pub package: String,
    pub prev: Option<RecordId>,
    pub entries: Vec<PackageEntryInfo>,
}

impl PublishInfo {
    pub fn new(package: String, prev: Option<RecordId>) -> Self {
        Self {
            package,
            prev,
            entries: vec![],
        }
    }

    pub fn push_init(&mut self, hash_algorithm: HashAlgorithm, key: signing::PublicKey) {
        let init = PackageEntryInfo::Init {
            hash_algorithm,
            key,
        };
        self.entries.push(init);
    }

    pub fn push_release(&mut self, version: Version, content: DynHash) {
        let release = PackageEntryInfo::Release { version, content };
        self.entries.push(release);
    }

    pub fn finalize(self) -> (String, Vec<DynHash>, package::PackageRecord) {
        let name = self.package;
        let content = self
            .entries
            .iter()
            .filter_map(|entry| match entry {
                PackageEntryInfo::Init { .. } => None,
                PackageEntryInfo::Release { content, .. } => Some(content.to_owned()),
            })
            .collect();
        let package = package::PackageRecord {
            prev: self.prev,
            version: package::PACKAGE_RECORD_VERSION,
            timestamp: SystemTime::now(),
            entries: self
                .entries
                .into_iter()
                .map(|entry| match entry {
                    PackageEntryInfo::Init {
                        hash_algorithm,
                        key,
                    } => package::PackageEntry::Init {
                        hash_algorithm,
                        key,
                    },
                    PackageEntryInfo::Release { version, content } => {
                        package::PackageEntry::Release { version, content }
                    }
                })
                .collect(),
        };
        (name, content, package)
    }
}

#[derive(Serialize, Deserialize)]
pub enum PackageEntryInfo {
    Init {
        hash_algorithm: HashAlgorithm,
        key: signing::PublicKey,
    },
    Release {
        version: Version,
        content: DynHash,
    },
}

#[async_trait]
pub trait ExpectedContent {
    /// Write new bytes of the content
    async fn write_all(&mut self, bytes: &[u8]) -> Result<()>;

    /// Finalize the content, storing it if the hash matched the expected value
    /// and returning an error if it didn't.
    async fn finalize(self: Box<Self>) -> Result<()>;
}

#[async_trait]
pub trait NewContent {
    async fn write_all(&mut self, bytes: &[u8]) -> Result<()>;

    async fn finalize(self: Box<Self>) -> Result<DynHash>;
}
