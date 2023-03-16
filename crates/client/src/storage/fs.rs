//! A module for file system client storage.

use super::{ClientStorage, PackageInfo, PublishInfo, RegistryInfo};
use crate::lock::FileLock;
use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{Stream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    pin::Pin,
};
use tempfile::NamedTempFile;
use tokio::io::{AsyncWriteExt, BufReader, BufWriter};
use tokio_util::io::ReaderStream;
use warg_crypto::hash::{Digest, DynHash, Hash, Sha256};
use warg_protocol::registry::LogId;

const TEMP_DIRECTORY: &str = "temp";
const CONTENTS_DIRECTORY: &str = "contents";
const PACKAGES_DIRECTORY: &str = "packages";
const REGISTRY_INFO_FILE: &str = "registry-info.json";
const PUBLISH_QUEUE_FILE: &str = "publish-queue.json";
const REGISTRY_LOCK_FILE: &str = ".lock";

/// Implements client storage using the local file system.
pub struct FileSystemStorage {
    _lock: FileLock,
    temp_dir: PathBuf,
    contents_dir: PathBuf,
    packages_dir: PathBuf,
    registry_info_path: PathBuf,
    publish_queue_path: PathBuf,
}

impl FileSystemStorage {
    /// Attempts to lock the file system storage.
    ///
    /// The base directory will be created if it does not exist.
    ///
    /// If the lock cannot be acquired, `Ok(None)` is returned.
    pub fn try_lock(base_dir: impl Into<PathBuf>) -> Result<Option<Self>> {
        let base_dir = base_dir.into();
        match FileLock::try_open_rw(base_dir.join(REGISTRY_LOCK_FILE))? {
            Some(lock) => Ok(Some(Self::new(base_dir, lock))),
            None => Ok(None),
        }
    }

    /// Locks a new file system storage at the given base directory.
    ///
    /// The base directory will be created if it does not exist.
    ///
    /// If the lock cannot be immediately acquired, this function
    /// will block.
    pub fn lock(base_dir: impl Into<PathBuf>) -> Result<Self> {
        let base_dir = base_dir.into();
        let lock = FileLock::open_rw(base_dir.join(REGISTRY_LOCK_FILE))?;
        Ok(Self::new(base_dir, lock))
    }

    fn new(base_dir: PathBuf, lock: FileLock) -> Self {
        let temp_dir = base_dir.join(TEMP_DIRECTORY);
        let contents_dir = base_dir.join(CONTENTS_DIRECTORY);
        let packages_dir = base_dir.join(PACKAGES_DIRECTORY);
        let registry_info_path = base_dir.join(REGISTRY_INFO_FILE);
        let publish_queue_path = base_dir.join(PUBLISH_QUEUE_FILE);

        Self {
            _lock: lock,
            temp_dir,
            contents_dir,
            packages_dir,
            registry_info_path,
            publish_queue_path,
        }
    }

    /// Creates a temporary file in the client storage.
    pub fn temp_file(&self) -> Result<NamedTempFile> {
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

    /// Gets the path to the registry information file.
    ///
    /// The path is not checked for existence.
    pub fn registry_info_path(&self) -> &Path {
        &self.registry_info_path
    }

    /// Gets the path to the publish queue file.
    ///
    /// The path is not checked for existence.
    pub fn publish_queue_path(&self) -> &Path {
        &self.publish_queue_path
    }

    /// Gets the path to the directory containing package contents.
    ///
    /// The path is not checked for existence.
    pub fn contents_dir(&self) -> &Path {
        &self.contents_dir
    }

    /// Gets the path to the content associated with the given digest.
    ///
    /// The path is not checked for existence.
    pub fn content_path(&self, digest: &DynHash) -> PathBuf {
        self.contents_dir()
            .join(digest.to_string().replace(':', "-"))
    }

    /// Gets the path to the directory containing package state.
    ///
    /// The path is not checked for existence.
    pub fn packages_dir(&self) -> &Path {
        &self.packages_dir
    }

    /// Gets the path to the package state for the given package name.
    ///
    /// The path is not checked for existence.
    pub fn package_path(&self, name: &str) -> PathBuf {
        self.packages_dir().join(
            LogId::package_log::<Sha256>(name)
                .to_string()
                .replace(':', "-"),
        )
    }
}

#[async_trait]
impl ClientStorage for FileSystemStorage {
    async fn load_registry_info(&self) -> Result<Option<RegistryInfo>> {
        load(self.registry_info_path()).await
    }

    async fn store_registry_info(&self, info: &RegistryInfo) -> Result<()> {
        store(self.registry_info_path(), info).await
    }

    async fn load_packages(&self) -> Result<Vec<PackageInfo>> {
        let mut packages = Vec::new();
        let dir = self.packages_dir();
        if !dir.exists() {
            return Ok(packages);
        }

        for entry in dir
            .read_dir()
            .with_context(|| format!("failed to read directory `{path}`", path = dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            packages.push(load(&path).await?.ok_or_else(|| {
                anyhow!(
                    "failed to load package state from `{path}`",
                    path = path.display()
                )
            })?);
        }

        Ok(packages)
    }

    async fn load_package_info(&self, package: &str) -> Result<Option<PackageInfo>> {
        Ok(load(&self.package_path(package)).await?)
    }

    async fn store_package_info(&self, info: &PackageInfo) -> Result<()> {
        store(&self.package_path(&info.name), info).await
    }

    fn has_publish_info(&self) -> bool {
        self.publish_queue_path().is_file()
    }

    async fn load_publish_info(&self) -> Result<Option<PublishInfo>> {
        Ok(load(self.publish_queue_path()).await?.unwrap_or_default())
    }

    async fn store_publish_info(&self, info: Option<&PublishInfo>) -> Result<()> {
        match info {
            Some(info) => store(self.publish_queue_path(), info).await,
            None => delete(self.publish_queue_path()).await,
        }
    }

    fn content_location(&self, digest: &DynHash) -> Option<PathBuf> {
        let path = self.content_path(digest);
        if path.is_file() {
            Some(path)
        } else {
            None
        }
    }

    async fn load_content(
        &self,
        digest: &DynHash,
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
        expected_digest: Option<&DynHash>,
    ) -> Result<DynHash> {
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

        let hash = DynHash::from(Hash::<Sha256>::from(hasher.finalize()));

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
