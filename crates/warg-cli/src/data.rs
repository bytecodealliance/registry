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
        let mut base = PathBuf::from(".");
        base.push(".warg");
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

    pub fn content_path(&self, digest: &DynHash) -> PathBuf {
        let sanitized = digest.to_string().replace(":", "-");

        let mut path = self.base.to_owned();
        path.push("content");
        path.push(sanitized);
        path
    }

    pub fn add_content(&self, path: &Path) -> Result<DynHash> {
        let contents = fs::read_to_string(path)?;
        let digest: Hash<Sha256> = Hash::of(&contents.as_str());
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

    fn package_path(&self, name: &str) -> PathBuf {
        let mut path = self.base.to_owned();
        path.push("package");
        path.push(name);
        path
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
    let contents = serde_json::to_string_pretty(value)?;
    dump(path, &contents)?;
    Ok(())
}

fn dump(path: &Path, contents: &str) -> Result<()> {
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
