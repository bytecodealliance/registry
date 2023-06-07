//! A module for file system client storage.

use super::{ContentStorage, OperatorInfo, PackageInfo, PublishInfo, RegistryStorage};
use crate::lock::FileLock;
use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{Stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    pin::Pin,
};
use tempfile::NamedTempFile;
use tokio::io::{AsyncWriteExt, BufReader, BufWriter};
use tokio_util::io::ReaderStream;
use walkdir::WalkDir;
use warg_crypto::hash::{AnyHash, Digest, Hash, Sha256};
use warg_protocol::{
    registry::{LogId, MapCheckpoint, PackageId},
    SerdeEnvelope,
};

const TEMP_DIRECTORY: &str = "temp";
const PENDING_PUBLISH_FILE: &str = "pending-publish.json";
const LOCK_FILE_NAME: &str = ".lock";

/// Represents a package storage using the local file system.
pub struct FileSystemRegistryStorage {
    _lock: FileLock,
    base_dir: PathBuf,
}

impl FileSystemRegistryStorage {
    /// Attempts to lock the package storage.
    ///
    /// The base directory will be created if it does not exist.
    ///
    /// If the lock cannot be acquired, `Ok(None)` is returned.
    pub fn try_lock(base_dir: impl Into<PathBuf>) -> Result<Option<Self>> {
        let base_dir = base_dir.into();
        match FileLock::try_open_rw(base_dir.join(LOCK_FILE_NAME))? {
            Some(lock) => Ok(Some(Self {
                _lock: lock,
                base_dir,
            })),
            None => Ok(None),
        }
    }

    /// Locks a new package storage at the given base directory.
    ///
    /// The base directory will be created if it does not exist.
    ///
    /// If the lock cannot be immediately acquired, this function
    /// will block.
    pub fn lock(base_dir: impl Into<PathBuf>) -> Result<Self> {
        let base_dir = base_dir.into();
        let lock = FileLock::open_rw(base_dir.join(LOCK_FILE_NAME))?;
        Ok(Self {
            _lock: lock,
            base_dir,
        })
    }

    fn operator_path(&self) -> PathBuf {
        self.base_dir.join("operator.log")
    }

    fn package_path(&self, id: &PackageId) -> PathBuf {
        self.base_dir.join(
            LogId::package_log::<Sha256>(id)
                .to_string()
                .replace(':', "/"),
        )
    }

    fn pending_publish_path(&self) -> PathBuf {
        self.base_dir.join(PENDING_PUBLISH_FILE)
    }
}

#[async_trait]
impl RegistryStorage for FileSystemRegistryStorage {
    async fn load_checkpoint(&self) -> Result<Option<SerdeEnvelope<MapCheckpoint>>> {
        load(&self.base_dir.join("checkpoint")).await
    }

    async fn store_checkpoint(&self, checkpoint: &SerdeEnvelope<MapCheckpoint>) -> Result<()> {
        store(&self.base_dir.join("checkpoint"), checkpoint).await
    }

    async fn load_packages(&self) -> Result<Vec<PackageInfo>> {
        let mut packages = Vec::new();

        for entry in WalkDir::new(&self.base_dir) {
            let entry = entry.with_context(|| {
                anyhow!(
                    "failed to walk directory `{path}`",
                    path = self.base_dir.display()
                )
            })?;

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            if let Some(name) = path.file_name().and_then(OsStr::to_str) {
                if name.starts_with('.') {
                    continue;
                }
            }

            packages.push(load(path).await?.ok_or_else(|| {
                anyhow!(
                    "failed to load package state from `{path}`",
                    path = path.display()
                )
            })?);
        }

        Ok(packages)
    }

    async fn load_operator(&self) -> Result<Option<OperatorInfo>> {
        Ok(load(&self.operator_path()).await?)
    }

    async fn store_operator(&self, info: OperatorInfo) -> Result<()> {
        store(&self.operator_path(), info).await
    }

    async fn load_package(&self, package: &PackageId) -> Result<Option<PackageInfo>> {
        Ok(load(&self.package_path(package)).await?)
    }

    async fn store_package(&self, info: &PackageInfo) -> Result<()> {
        store(&self.package_path(&info.id), info).await
    }

    async fn load_publish(&self) -> Result<Option<PublishInfo>> {
        Ok(load(&self.base_dir.join(PENDING_PUBLISH_FILE))
            .await?
            .unwrap_or_default())
    }

    async fn store_publish(&self, info: Option<&PublishInfo>) -> Result<()> {
        let path = self.pending_publish_path();
        match info {
            Some(info) => store(&path, info).await,
            None => delete(&path).await,
        }
    }
}

/// Represents a content storage using the local file system.
pub struct FileSystemContentStorage {
    _lock: FileLock,
    base_dir: PathBuf,
    temp_dir: PathBuf,
}

impl FileSystemContentStorage {
    /// Attempts to lock the content storage.
    ///
    /// The base directory will be created if it does not exist.
    ///
    /// If the lock cannot be acquired, `Ok(None)` is returned.
    pub fn try_lock(base_dir: impl Into<PathBuf>) -> Result<Option<Self>> {
        let base_dir = base_dir.into();
        let temp_dir = base_dir.join(TEMP_DIRECTORY);
        match FileLock::try_open_rw(base_dir.join(LOCK_FILE_NAME))? {
            Some(lock) => Ok(Some(Self {
                _lock: lock,
                base_dir,
                temp_dir,
            })),
            None => Ok(None),
        }
    }

    /// Locks a new content storage at the given base directory.
    ///
    /// The base directory will be created if it does not exist.
    ///
    /// If the lock cannot be immediately acquired, this function
    /// will block.
    pub fn lock(base_dir: impl Into<PathBuf>) -> Result<Self> {
        let base_dir = base_dir.into();
        let temp_dir = base_dir.join(TEMP_DIRECTORY);
        let lock = FileLock::open_rw(base_dir.join(LOCK_FILE_NAME))?;
        Ok(Self {
            _lock: lock,
            base_dir,
            temp_dir,
        })
    }

    fn temp_file(&self) -> Result<NamedTempFile> {
        fs::create_dir_all(&self.temp_dir).with_context(|| {
            format!(
                "failed to create directory `{path}`",
                path = self.temp_dir.display()
            )
        })?;

        NamedTempFile::new_in(&self.temp_dir).with_context(|| {
            format!(
                "failed to create temporary file in `{path}`",
                path = self.temp_dir.display()
            )
        })
    }

    fn content_path(&self, digest: &AnyHash) -> PathBuf {
        self.base_dir.join(digest.to_string().replace(':', "/"))
    }
}

#[async_trait]
impl ContentStorage for FileSystemContentStorage {
    fn content_location(&self, digest: &AnyHash) -> Option<PathBuf> {
        let path = self.content_path(digest);
        if path.is_file() {
            Some(path)
        } else {
            None
        }
    }

    async fn load_content(
        &self,
        digest: &AnyHash,
    ) -> Result<Option<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send + Sync>>>> {
        let path = self.content_path(digest);
        if !path.is_file() {
            return Ok(None);
        }

        Ok(Some(Box::pin(
            ReaderStream::new(BufReader::new(
                tokio::fs::File::open(&path)
                    .await
                    .with_context(|| format!("failed to open `{path}`", path = path.display()))?,
            ))
            .map_err(|e| anyhow!(e)),
        )))
    }

    async fn store_content(
        &self,
        mut stream: Pin<Box<dyn Stream<Item = Result<Bytes>> + Send + Sync>>,
        expected_digest: Option<&AnyHash>,
    ) -> Result<AnyHash> {
        let (file, path) = self.temp_file()?.into_parts();
        let mut writer = BufWriter::new(tokio::fs::File::from_std(file));
        let mut hasher = Sha256::new();

        while let Some(bytes) = stream.next().await.transpose()? {
            hasher.update(&bytes);
            writer
                .write_all(&bytes)
                .await
                .with_context(|| format!("failed to write to `{path}`", path = path.display()))?;
        }

        let hash = AnyHash::from(Hash::<Sha256>::from(hasher.finalize()));

        if let Some(expected) = expected_digest {
            if hash != *expected {
                bail!(
                    "stored content has digest `{hash}` but a digest of `{expected}` was expected",
                );
            }
        }

        writer
            .shutdown()
            .await
            .with_context(|| format!("failed to write `{path}`", path = path.display()))?;

        drop(writer);

        let content_path = self.content_path(&hash);
        if !content_path.is_file() {
            if let Some(parent) = content_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "failed to create directory `{path}`",
                        path = parent.display()
                    )
                })?;
            }

            path.persist(&content_path).with_context(|| {
                format!(
                    "failed to persist temporary file to `{path}`",
                    path = content_path.display()
                )
            })?;
        }

        Ok(hash)
    }
}

async fn load<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<Option<T>> {
    if !path.is_file() {
        return Ok(None);
    }

    let contents = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("failed to read `{path}`", path = path.display()))?;

    serde_json::from_str(&contents).with_context(|| {
        format!(
            "failed to deserialize contents of `{path}`",
            path = path.display()
        )
    })
}

async fn store(path: &Path, value: impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create parent directory for `{path}`",
                path = path.display()
            )
        })?;
    }

    let contents = serde_json::to_vec_pretty(&value).with_context(|| {
        format!(
            "failed to serialize contents of `{path}`",
            path = path.display()
        )
    })?;

    tokio::fs::write(path, contents)
        .await
        .with_context(|| format!("failed to write `{path}`", path = path.display()))
}

async fn delete(path: &Path) -> Result<()> {
    if path.is_file() {
        tokio::fs::remove_file(path)
            .await
            .with_context(|| format!("failed to delete file `{path}`", path = path.display()))?;
    }

    Ok(())
}
