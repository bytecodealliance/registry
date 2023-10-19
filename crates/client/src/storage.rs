//! A module for client storage implementations.

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, pin::Pin, time::SystemTime};
use warg_crypto::{
    hash::{AnyHash, HashAlgorithm},
    signing::{self, KeyID, PublicKey},
};
use warg_protocol::{
    operator,
    package::{self, PackageRecord, Permission, PACKAGE_RECORD_VERSION},
    registry::{Checkpoint, PackageId, RecordId, RegistryIndex, TimestampedCheckpoint},
    ProtoEnvelope, SerdeEnvelope, Version,
};

mod fs;
pub use fs::*;

/// Trait for registry storage implementations.
///
/// Stores information such as package/operator logs and checkpoints
/// on a per-registry basis.
///
/// Registry storage data must be synchronized if shared between
/// multiple threads and processes.
#[async_trait]
pub trait RegistryStorage: Send + Sync {
    /// Reset registry local data
    async fn reset(&self, all_registries: bool) -> Result<()>;

    /// Loads most recent checkpoint
    async fn load_checkpoint(&self) -> Result<Option<SerdeEnvelope<TimestampedCheckpoint>>>;

    /// Stores most recent checkpoint
    async fn store_checkpoint(
        &self,
        ts_checkpoint: &SerdeEnvelope<TimestampedCheckpoint>,
    ) -> Result<()>;

    /// Loads the operator information from the storage.
    ///
    /// Returns `Ok(None)` if the information is not present.
    async fn load_operator(&self) -> Result<Option<OperatorInfo>>;

    /// Stores the operator information in the storage.
    async fn store_operator(&self, operator: OperatorInfo) -> Result<()>;

    /// Loads the package information for all packages in the storage.
    async fn load_packages(&self) -> Result<Vec<PackageInfo>>;

    /// Loads the package information from the storage.
    ///
    /// Returns `Ok(None)` if the information is not present.
    async fn load_package(&self, package: &PackageId) -> Result<Option<PackageInfo>>;

    /// Stores the package information in the storage.
    async fn store_package(&self, info: &PackageInfo) -> Result<()>;

    /// Loads information about a pending publish operation.
    ///
    /// Returns `Ok(None)` if the information is not present.
    async fn load_publish(&self) -> Result<Option<PublishInfo>>;

    /// Stores information about a pending publish operation.
    ///
    /// If the info is `None`, the any existing publish information is deleted.
    async fn store_publish(&self, info: Option<&PublishInfo>) -> Result<()>;
}

/// Trait for content storage implementations.
///
/// Content storage data must be synchronized if shared between
/// multiple threads and processes.
#[async_trait]
pub trait ContentStorage: Send + Sync {
    /// Clear content local data
    async fn clear(&self) -> Result<()>;

    /// Gets the location of the content associated with the given digest if it
    /// exists as a file on disk.
    ///
    /// Returns `None` if the content is not present on disk.
    fn content_location(&self, digest: &AnyHash) -> Option<PathBuf>;

    /// Loads the content associated with the given digest as a stream.
    ///
    /// If the content is not found, `Ok(None)` is returned.
    async fn load_content(
        &self,
        digest: &AnyHash,
    ) -> Result<Option<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send + Sync>>>>;

    /// Stores the given stream as content.
    ///
    /// If `expected_digest` is `Some`, the storage will verify that the written
    /// content matches the given digest. If the digests do not match, an
    /// error is returned.
    ///
    /// Returns the hash of the written content.
    async fn store_content(
        &self,
        stream: Pin<Box<dyn Stream<Item = Result<Bytes>> + Send + Sync>>,
        expected_digest: Option<&AnyHash>,
    ) -> Result<AnyHash>;
}

/// Represents information about a registry operator.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OperatorInfo {
    /// The current operator log state
    #[serde(default)]
    pub state: operator::LogState,
    /// The registry log index of the most recent record
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_registry_index: Option<RegistryIndex>,
    /// The fetch token for the most recent record
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_fetch_token: Option<String>,
}

/// Represents information about a registry package.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInfo {
    /// The id of the package to publish.
    pub id: PackageId,
    /// The last known checkpoint of the package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<Checkpoint>,
    /// The current package log state
    #[serde(default)]
    pub state: package::LogState,
    /// The registry log index of the most recent record
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_registry_index: Option<RegistryIndex>,
    /// The fetch token for the most recent record
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_fetch_token: Option<String>,
}

impl PackageInfo {
    /// Creates a new package info for the given package id.
    pub fn new(id: impl Into<PackageId>) -> Self {
        Self {
            id: id.into(),
            checkpoint: None,
            state: package::LogState::default(),
            head_registry_index: None,
            head_fetch_token: None,
        }
    }
}

/// Represents a record entry being published.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PublishEntry {
    /// The package is being initialized.
    Init,
    /// A new release entry is being published.
    Release {
        /// The version of the release.
        version: Version,
        /// The content digest of the release.
        content: AnyHash,
    },
    /// A release is being yanked.
    Yank {
        /// The version of the release being yanked.
        version: Version,
    },
    /// A key is being granted permission(s).
    Grant {
        /// The public key being granted to.
        key: PublicKey,
        /// The permission(s) being granted.
        permissions: Vec<Permission>,
    },
    /// A key's permission(s) are being revoked.
    Revoke {
        /// The key ID being revoked from.
        key_id: KeyID,
        /// The permission(s) being revoked.
        permissions: Vec<Permission>,
    },
}

/// Represents information about a package publish.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishInfo {
    /// The id of the package being published.
    pub id: PackageId,
    /// The last known head of the package log to use.
    ///
    /// If `None` and the package is not being initialized,
    /// the latest head of the package log will be fetched prior to publishing.
    pub head: Option<RecordId>,
    /// The new record entries to publish.
    pub entries: Vec<PublishEntry>,
}

impl PublishInfo {
    /// Determines if the publish information is initializing the package.
    pub fn initializing(&self) -> bool {
        self.entries.iter().any(|e| matches!(e, PublishEntry::Init))
    }

    pub(crate) fn finalize(
        self,
        signing_key: &signing::PrivateKey,
    ) -> Result<ProtoEnvelope<PackageRecord>> {
        let mut entries = Vec::with_capacity(self.entries.len());
        for entry in self.entries {
            match entry {
                PublishEntry::Init => {
                    entries.push(package::PackageEntry::Init {
                        hash_algorithm: HashAlgorithm::Sha256,
                        key: signing_key.public_key(),
                    });
                }
                PublishEntry::Release { version, content } => {
                    entries.push(package::PackageEntry::Release { version, content });
                }
                PublishEntry::Yank { version } => {
                    entries.push(package::PackageEntry::Yank { version })
                }
                PublishEntry::Grant { key, permissions } => {
                    entries.push(package::PackageEntry::GrantFlat { key, permissions })
                }
                PublishEntry::Revoke {
                    key_id,
                    permissions,
                } => entries.push(package::PackageEntry::RevokeFlat {
                    key_id,
                    permissions,
                }),
            }
        }

        let record = package::PackageRecord {
            prev: self.head,
            version: PACKAGE_RECORD_VERSION,
            // TODO: this seems wrong to record the current time client-side
            // How can we guarantee that the timestamps are monotonic?
            // Should incrementing timestamps even be a requirement?
            timestamp: SystemTime::now(),
            entries,
        };

        Ok(ProtoEnvelope::signed_contents(signing_key, record)?)
    }
}
