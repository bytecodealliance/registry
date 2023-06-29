//! Commands for the `warg` tool.

use anyhow::Context;
use anyhow::Result;
use clap::Args;
use std::path::PathBuf;
use url::Url;
use warg_client::{ClientError, Config, FileSystemClient, StorageLockResult};
use warg_crypto::signing::PrivateKey;

mod config;
mod download;
mod info;
mod key;
mod publish;
mod run;
mod update;

use crate::keyring::get_signing_key;

pub use self::config::*;
pub use self::download::*;
pub use self::info::*;
pub use self::key::*;
pub use self::publish::*;
pub use self::run::*;
pub use self::update::*;

/// Common options for commands.
#[derive(Args)]
pub struct CommonOptions {
    /// The URL of the registry to use.
    #[clap(long, value_name = "URL")]
    pub registry: Option<String>,
    /// The name to use for the signing key.
    #[clap(long, short, value_name = "KEY_NAME", default_value = "default")]
    pub key_name: String,
    /// The path to the signing key file.
    #[clap(long, value_name = "KEY_FILE", env = "WARG_SIGNING_KEY_FILE")]
    pub key_file: Option<PathBuf>,
    /// The path to the client configuration file to use.
    ///
    /// If not specified, the following locations are searched in order: `./warg-config.json`, `<system-config-dir>/warg/config.json`.
    ///
    /// If no configuration file is found, a default configuration is used.
    #[clap(long, value_name = "CONFIG")]
    pub config: Option<PathBuf>,
}

impl CommonOptions {
    /// Reads the client configuration.
    ///
    /// If a client configuration was not specified, a default configuration is returned.
    pub fn read_config(&self) -> Result<Config> {
        Ok(self
            .config
            .as_ref()
            .map_or_else(Config::from_default_file, |p| {
                Config::from_file(p).map(Some)
            })?
            .unwrap_or_default())
    }

    /// Creates the warg client to use.
    pub fn create_client(&self, config: &Config) -> Result<FileSystemClient, ClientError> {
        match FileSystemClient::try_new_with_config(self.registry.as_deref(), config)? {
            StorageLockResult::Acquired(client) => Ok(client),
            StorageLockResult::NotAcquired(path) => {
                println!(
                    "blocking on lock for directory `{path}`...",
                    path = path.display()
                );

                FileSystemClient::new_with_config(self.registry.as_deref(), config)
            }
        }
    }

    /// Gets the signing key for the given registry URL.
    pub fn signing_key(&self, registry_url: &str) -> Result<PrivateKey> {
        if let Some(file) = &self.key_file {
            let key_str = std::fs::read_to_string(file)
                .with_context(|| format!("failed to read key from {file:?}"))?
                .trim_end()
                .to_string();
            PrivateKey::decode(key_str)
                .with_context(|| format!("failed to parse key from {file:?}"))
        } else {
            let url: Url = registry_url
                .parse()
                .with_context(|| format!("failed to parse registry URL `{registry_url}`"))?;

            let host = url
                .host_str()
                .with_context(|| format!("registry URL `{url}` has no host"))?;

            get_signing_key(host, &self.key_name)
        }
    }
}
