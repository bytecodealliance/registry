//! Module for client configuration.

use crate::{ClientError, RegistryUrl};
use anyhow::{anyhow, Context, Result};
use normpath::PathExt;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    env::current_dir,
    fs::{self, File},
    path::{Component, Path, PathBuf},
};

static CACHE_DIR: Lazy<Option<PathBuf>> = Lazy::new(dirs::cache_dir);
static CONFIG_DIR: Lazy<Option<PathBuf>> = Lazy::new(dirs::config_dir);
static CONFIG_FILE_NAME: &str = "warg-config.json";

fn find_warg_config(cwd: &Path) -> Option<PathBuf> {
    let mut current = Some(cwd);

    while let Some(dir) = current {
        let config = dir.join(CONFIG_FILE_NAME);
        if config.is_file() {
            return Some(config);
        }

        current = dir.parent();
    }

    None
}

/// Normalize a path, removing things like `.` and `..`.
/// Sourced from: https://github.com/rust-lang/cargo/blob/15d090969743630bff549a1b068bcaa8174e5ee3/crates/cargo-util/src/paths.rs#L82
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

/// Paths used for storage
pub struct StoragePaths {
    /// The registry URL relating to the storage paths.
    pub registry_url: RegistryUrl,
    /// The path to the registry storage directory.
    pub registries_dir: PathBuf,
    /// The path to the content storage directory.
    pub content_dir: PathBuf,
}

/// Represents the Warg client configuration.
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// The default Warg registry server URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_url: Option<String>,

    /// The path to the top-level directory where per-registry information is stored.
    ///
    /// This path is expected to be relative to the configuration file.
    ///
    /// If `None`, the default of `$CACHE_DIR/warg/registries` is used, where
    /// `$CACHE_DIR` is the platform-specific cache directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registries_dir: Option<PathBuf>,

    /// The path to the directory where package content is stored.
    ///
    /// This path is expected to be relative to the configuration file.
    ///
    /// If `None`, the default of `$CACHE_DIR/warg/content` is used, where
    /// `$CACHE_DIR` is the platform-specific cache directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_dir: Option<PathBuf>,
}

impl Config {
    /// Reads the client configuration from the given file path.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let config = fs::read_to_string(path).with_context(|| {
            format!(
                "failed to read configuration file `{path}`",
                path = path.display()
            )
        })?;

        let mut config: Self = serde_json::from_str(&config).with_context(|| {
            format!("failed to deserialize file `{path}`", path = path.display())
        })?;

        if let Some(parent) = path.parent() {
            config.registries_dir = config.registries_dir.map(|p| parent.join(p));
            config.content_dir = config.content_dir.map(|p| parent.join(p));
        }

        Ok(config)
    }

    /// Writes the client configuration to the given file path.
    ///
    /// This function will normalize the paths in the configuration file to be
    /// relative to the configuration file's directory.
    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let current_dir = current_dir().context("failed to get current directory")?;
        let path = current_dir.join(path);
        let parent = path.parent().ok_or_else(|| {
            anyhow!(
                "path `{path}` has no parent directory",
                path = path.display()
            )
        })?;

        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create parent directory `{path}`",
                path = parent.display()
            )
        })?;

        // We must normalize the parent directory for forming relative paths
        // This is used to get the actual path of the configuration file
        // directory; below we use `normalize_path` as the directories might
        // not exist.
        let parent = parent.normalize().with_context(|| {
            format!(
                "failed to normalize parent directory `{path}`",
                path = parent.display()
            )
        })?;

        assert!(parent.is_absolute());

        let config = Config {
            default_url: self.default_url.clone(),
            registries_dir: self.registries_dir.as_ref().map(|p| {
                let p = normalize_path(parent.join(p).as_path());
                assert!(p.is_absolute());
                pathdiff::diff_paths(&p, &parent).unwrap()
            }),
            content_dir: self.content_dir.as_ref().map(|p| {
                let p = normalize_path(parent.join(p).as_path());
                assert!(p.is_absolute());
                pathdiff::diff_paths(&p, &parent).unwrap()
            }),
        };

        serde_json::to_writer_pretty(
            File::create(&path).with_context(|| {
                format!("failed to create file `{path}`", path = path.display())
            })?,
            &config,
        )
        .with_context(|| format!("failed to serialize file `{path}`", path = path.display()))
    }

    /// Loads a client configuration from a default file path.
    ///
    /// The following paths are checked in order:
    ///
    /// * `warg-config.json` at the current directory and its parents
    /// * `$CONFIG_DIR/warg/config.json`
    ///
    /// Where `$CONFIG_DIR` is the platform-specific configuration directory.
    ///
    /// Returns `Ok(None)` if no configuration file was found.
    pub fn from_default_file() -> Result<Option<Self>> {
        if let Some(path) = find_warg_config(&std::env::current_dir()?) {
            return Ok(Some(Self::from_file(path)?));
        }

        let path = Self::default_config_path()?;
        if path.is_file() {
            return Ok(Some(Self::from_file(path)?));
        }

        Ok(None)
    }

    /// Gets the path to the default configuration file.
    ///
    /// The default configuration file is `$CONFIG_DIR/warg/config.json`,
    pub fn default_config_path() -> Result<PathBuf> {
        CONFIG_DIR
            .as_ref()
            .map(|p| p.join("warg/config.json"))
            .ok_or_else(|| anyhow!("failed to determine operating system configuration directory"))
    }

    /// Gets the path to the directory where per-registry packages are stored.
    pub fn registries_dir(&self) -> Result<PathBuf> {
        self.registries_dir
            .as_ref()
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| {
                CACHE_DIR
                    .as_ref()
                    .map(|p| p.join("warg/registries"))
                    .ok_or_else(|| anyhow!("failed to determine operating system cache directory"))
            })
    }

    /// Gets the path to the directory where per-registry packages are stored.
    pub fn content_dir(&self) -> Result<PathBuf> {
        self.content_dir
            .as_ref()
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| {
                CACHE_DIR
                    .as_ref()
                    .map(|p| p.join("warg/content"))
                    .ok_or_else(|| anyhow!("failed to determine operating system cache directory"))
            })
    }

    pub(crate) fn storage_paths_for_url(
        &self,
        url: Option<&str>,
    ) -> Result<StoragePaths, ClientError> {
        let registry_url = RegistryUrl::new(
            url.or(self.default_url.as_deref())
                .ok_or(ClientError::NoDefaultUrl)?,
        )?;

        let label = registry_url.safe_label();
        let registries_dir = self.registries_dir()?.join(label);
        let content_dir = self.content_dir()?;
        Ok(StoragePaths {
            registry_url,
            registries_dir,
            content_dir,
        })
    }
}
