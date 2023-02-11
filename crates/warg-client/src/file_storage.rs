use std::path::{Path, PathBuf};

use anyhow::{Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tempfile::{NamedTempFile, TempPath};
use tokio::io::AsyncWriteExt;
use warg_crypto::hash::{Digest, DynHash, Hash, Sha256};
use warg_protocol::package;

use crate::storage::{ClientStorage, ExpectedContent, NewContent, PublishInfo, RegistryInfo};

pub struct FileSystemStorage {
    base: PathBuf,
    publish_info: PathBuf,
    registry_info: PathBuf,
}

impl FileSystemStorage {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut base = std::env::current_dir().unwrap();
        base.push(".warg");
        Self::ensure_dir(&base);

        let mut publish_info = base.clone();
        publish_info.push("publish-info.json");
        let mut registry_info = base.clone();
        registry_info.push("registry-info.json");
        Self {
            base,
            publish_info,
            registry_info,
        }
    }

    fn ensure_dir(dir: &Path) {
        if !dir.exists() {
            std::fs::create_dir_all(dir).unwrap();
        }
    }

    pub fn temp_dir(&self) -> PathBuf {
        let mut path = self.base.to_owned();
        path.push("temp");
        Self::ensure_dir(&path);
        path
    }

    fn content_dir(&self) -> PathBuf {
        let mut path = self.base.to_owned();
        path.push("content");
        Self::ensure_dir(&path);
        path
    }

    pub fn content_path(&self, digest: &DynHash) -> PathBuf {
        let sanitized = digest.to_string().replace(':', "-");

        let mut path = self.content_dir();
        path.push(sanitized);
        path
    }

    fn package_dir(&self) -> PathBuf {
        let mut path = self.base.to_owned();
        path.push("package");
        Self::ensure_dir(&path);
        path
    }

    fn package_path(&self, name: &str) -> PathBuf {
        let mut path = self.package_dir();
        path.push(name);
        path
    }
}

#[async_trait]
impl ClientStorage for FileSystemStorage {
    async fn load_registry_info(&self) -> Result<Option<RegistryInfo>> {
        load(&self.registry_info).await
    }

    async fn store_registry_info(&mut self, info: &RegistryInfo) -> Result<()> {
        store(&self.registry_info, info).await
    }

    async fn load_publish_info(&self) -> Result<Option<PublishInfo>> {
        load(&self.publish_info).await
    }

    async fn store_publish_info(&mut self, info: &PublishInfo) -> Result<()> {
        store(&self.publish_info, &info).await
    }

    async fn clear_publish_info(&mut self) -> Result<()> {
        delete(&self.publish_info).await
    }

    async fn list_all_packages(&self) -> Result<Vec<String>> {
        let mut packages = Vec::new();
        for entry in self.package_dir().read_dir()? {
            let entry = entry?;
            let name = entry.file_name().to_str().unwrap().to_owned();
            packages.push(name);
        }
        Ok(packages)
    }

    async fn load_package_state(&self, package: &str) -> Result<package::Validator> {
        load_or_default(&self.package_path(package)).await
    }

    async fn store_package_state(
        &mut self,
        package: &str,
        state: &package::Validator,
    ) -> Result<()> {
        store(&self.package_path(package), state).await
    }

    async fn has_content(&self, digest: &DynHash) -> Result<bool> {
        Ok(self.content_path(digest).is_file())
    }

    async fn store_content<'s>(
        &'s mut self,
        digest: DynHash,
    ) -> Result<Box<dyn ExpectedContent + 's>> {
        let tmp_path = NamedTempFile::new_in(self.temp_dir())?.into_temp_path();
        let file = tokio::fs::File::create(&tmp_path).await?;
        Ok(Box::new(ExpectedFileContent {
            expected_hash: digest,
            storage: self,
            hasher: Sha256::new(),
            tmp_path,
            file,
        }))
    }

    async fn create_content<'s>(&'s mut self) -> Result<Box<dyn NewContent + 's>> {
        let tmp_path = NamedTempFile::new_in(self.temp_dir())?.into_temp_path();
        let file = tokio::fs::File::create(&tmp_path).await?;
        Ok(Box::new(NewFileContent {
            storage: self,
            hasher: Sha256::new(),
            tmp_path,
            file,
        }))
    }

    async fn get_content(&self, digest: &DynHash) -> Result<Option<Vec<u8>>> {
        let path = self.content_path(digest);
        if path.is_file() {
            Ok(Some(tokio::fs::read(path).await?))
        } else {
            Ok(None)
        }
    }
}

struct NewFileContent<'storage> {
    storage: &'storage mut FileSystemStorage,
    hasher: Sha256,
    tmp_path: TempPath,
    file: tokio::fs::File,
}

#[async_trait]
impl<'storage> NewContent for NewFileContent<'storage> {
    async fn write_all(&mut self, bytes: &[u8]) -> Result<()> {
        self.hasher.update(bytes);
        self.file.write_all(bytes).await?;
        Ok(())
    }

    async fn finalize(self: Box<Self>) -> Result<DynHash> {
        let hash = self.hasher.finalize();
        let hash: Hash<Sha256> = hash.into();
        let hash: DynHash = hash.into();
        if !self.storage.has_content(&hash).await? {
            let path = self.storage.content_path(&hash);
            self.tmp_path.persist(path)?;
        }
        Ok(hash)
    }
}

struct ExpectedFileContent<'storage> {
    expected_hash: DynHash,
    storage: &'storage mut FileSystemStorage,
    hasher: Sha256,
    tmp_path: TempPath,
    file: tokio::fs::File,
}

#[async_trait]
impl<'storage> ExpectedContent for ExpectedFileContent<'storage> {
    async fn write_all(&mut self, bytes: &[u8]) -> Result<()> {
        self.hasher.update(bytes);
        self.file.write_all(bytes).await?;
        Ok(())
    }

    async fn finalize(self: Box<Self>) -> Result<()> {
        let hash = self.hasher.finalize();
        let hash: Hash<Sha256> = hash.into();
        let hash: DynHash = hash.into();
        if hash == self.expected_hash {
            if !self.storage.has_content(&self.expected_hash).await? {
                let path = self.storage.content_path(&self.expected_hash);
                self.tmp_path.persist(path)?;
            }
            Ok(())
        } else {
            Err(Error::msg("Downloaded content digest did not match"))
        }
    }
}

async fn load<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<Option<T>> {
    if path.is_file() {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|_| Error::msg(format!("Info at path {:?} is malformed", &path)))
    } else {
        Ok(None)
    }
}

async fn load_or_default<T: Default + for<'a> Deserialize<'a>>(path: &Path) -> Result<T> {
    if path.is_file() {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|_| Error::msg(format!("Info at path {:?} is malformed", &path)))
    } else {
        Ok(Default::default())
    }
}

async fn store<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let contents = serde_json::to_vec_pretty(value)?;
    dump(path, &contents).await?;
    Ok(())
}

async fn dump(path: &Path, contents: &[u8]) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    if let Some(parent) = path.parent() {
        if parent.is_file() {
            return Err(Error::msg("Parent directory is file"));
        }
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, contents)?;
    Ok(())
}

async fn delete(path: &Path) -> Result<()> {
    std::fs::remove_file(path)?;
    Ok(())
}
