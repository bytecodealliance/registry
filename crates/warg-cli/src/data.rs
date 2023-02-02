use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{publish::PublishInfo, registry_info::RegistryInfo};
use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use warg_crypto::hash::{DynHash, Hash, Sha256};
use warg_protocol::package;

pub struct CliData {
    base: PathBuf,
    publish_info: PathBuf,
    registry_info: PathBuf,
}

impl CliData {
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
            std::fs::create_dir_all(&dir).unwrap();
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
        let sanitized = digest.to_string().replace(":", "-");

        let mut path = self.content_dir();
        path.push(sanitized);
        path
    }

    pub fn add_content(&self, path: &Path) -> Result<DynHash> {
        let contents = fs::read(path)?;
        let digest: Hash<Sha256> = Hash::of(contents.as_slice());
        let digest: DynHash = digest.into();
        let content_path = self.content_path(&digest);
        dump(&content_path, &contents)?;
        Ok(digest)
    }

    pub fn get_publish_info(&self) -> Result<Option<PublishInfo>> {
        load(&self.publish_info)
    }

    pub fn set_publish_info(&self, publish_info: &PublishInfo) -> Result<()> {
        store(&self.publish_info, &publish_info)
    }

    pub fn clear_publish_info(&self) -> Result<()> {
        delete(&self.publish_info)
    }

    pub fn get_registry_info(&self) -> Result<Option<RegistryInfo>> {
        load(&self.registry_info)
    }

    pub fn set_registry_info(&self, registry_info: &RegistryInfo) -> Result<()> {
        store(&self.registry_info, registry_info)
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

    pub fn get_all_packages(&self) -> Result<Vec<(String, package::Validator)>> {
        let mut packages = Vec::new();
        for entry in self.package_dir().read_dir()?.into_iter() {
            let entry = entry?;
            let name = entry.file_name().to_str().unwrap().to_owned();
            packages.push((name, load(&entry.path())?.unwrap()))
        }
        Ok(packages)
    }

    pub fn get_package_state(&self, name: &str) -> Result<package::Validator> {
        load_or_default(&self.package_path(name))
    }

    pub fn set_package_state(&self, name: &str, package_state: &package::Validator) -> Result<()> {
        store(&self.package_path(name), package_state)
    }
}

fn load<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<Option<T>> {
    if path.is_file() {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|_| Error::msg(format!("Info at path {:?} is malformed", &path)))
    } else {
        Ok(None)
    }
}

fn load_or_default<T: Default + for<'a> Deserialize<'a>>(path: &Path) -> Result<T> {
    if path.is_file() {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map_err(|_| Error::msg(format!("Info at path {:?} is malformed", &path)))
    } else {
        Ok(Default::default())
    }
}

fn store<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let contents = serde_json::to_vec_pretty(value)?;
    dump(path, &contents)?;
    Ok(())
}

fn dump(path: &Path, contents: &[u8]) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(&path)?;
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

fn delete(path: &Path) -> Result<()> {
    std::fs::remove_file(&path)?;
    Ok(())
}
