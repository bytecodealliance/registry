//! Commands for the `warg` tool.

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;
use warg_client::keyring::Keyring;
use warg_client::storage::RegistryDomain;
use warg_client::{ClientError, Config, FileSystemClient, StorageLockResult};
use warg_crypto::signing::PrivateKey;

mod bundle;
mod clear;
mod config;
mod dependencies;
mod download;
mod info;
mod key;
mod lock;
mod login;
mod logout;
mod publish;
mod reset;
mod update;

pub use self::bundle::*;
pub use self::clear::*;
pub use self::config::*;
pub use self::dependencies::*;
pub use self::download::*;
pub use self::info::*;
pub use self::key::*;
pub use self::lock::*;
pub use self::login::*;
pub use self::logout::*;
pub use self::publish::*;
pub use self::reset::*;
pub use self::update::*;

/// Common options for commands.
#[derive(Args)]
pub struct CommonOptions {
    /// The URL of the registry to use.
    #[clap(long, value_name = "URL")]
    pub registry: Option<String>,
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
        let client =
            match FileSystemClient::try_new_with_config(self.registry.as_deref(), config, None)? {
                StorageLockResult::Acquired(client) => Ok(client),
                StorageLockResult::NotAcquired(path) => {
                    println!(
                        "blocking on lock for directory `{path}`...",
                        path = path.display()
                    );

                    FileSystemClient::new_with_config(self.registry.as_deref(), config, None)
                }
            }?;
        Ok(client)
    }

    /// Gets the signing key for the given registry URL.
    pub async fn signing_key(
        &self,
        registry_domain: Option<&RegistryDomain>,
    ) -> Result<PrivateKey> {
        let config = self.read_config()?;
        Keyring::default().get_signing_key(
            registry_domain.map(|domain| domain.to_string()).as_deref(),
            &config.keys,
            config.home_url.as_deref(),
        )
    }
}
