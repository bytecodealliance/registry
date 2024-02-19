//! Commands for the `warg` tool.

use anyhow::Context;
use anyhow::Result;
use clap::Args;
use secrecy::Secret;
use std::path::PathBuf;
use warg_client::storage::ContentStorage;
use warg_client::storage::NamespaceMapStorage;
use warg_client::storage::RegistryStorage;
use warg_client::Client;
use warg_client::RegistryUrl;
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
mod publish;
mod reset;
mod update;

use crate::keyring::get_auth_token;
use crate::keyring::get_signing_key;

pub use self::bundle::*;
pub use self::clear::*;
pub use self::config::*;
pub use self::dependencies::*;
pub use self::download::*;
pub use self::info::*;
pub use self::key::*;
pub use self::lock::*;
pub use self::login::*;
pub use self::publish::*;
pub use self::reset::*;
pub use self::update::*;

/// Common options for commands.
#[derive(Args)]
pub struct CommonOptions {
    /// The URL of the registry to use.
    #[clap(long, value_name = "URL")]
    pub registry: Option<String>,
    /// The name to use for the auth token.
    #[clap(long, short, value_name = "TOKEN_NAME", default_value = "default")]
    pub token_name: String,
    /// The path to the auth token file.
    #[clap(long, value_name = "TOKEN_FILE", env = "WARG_AUTH_TOKEN_FILE")]
    pub token_file: Option<PathBuf>,
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
        match FileSystemClient::try_new_with_config(
            self.registry.as_deref(),
            config,
            self.auth_token(config)?.map(|tok| Secret::from(tok)),
        )? {
            StorageLockResult::Acquired(client) => Ok(client),
            StorageLockResult::NotAcquired(path) => {
                println!(
                    "blocking on lock for directory `{path}`...",
                    path = path.display()
                );

                FileSystemClient::new_with_config(
                    self.registry.as_deref(),
                    config,
                    self.auth_token(config)?.map(|tok| Secret::from(tok)),
                )
            }
        }
    }

    /// Gets the signing key for the given registry URL.
    pub fn signing_key(&self, registry_url: &RegistryUrl, config: &Config) -> Result<PrivateKey> {
        if let Some(file) = &self.key_file {
            let key_str = std::fs::read_to_string(file)
                .with_context(|| format!("failed to read key from {file:?}"))?
                .trim_end()
                .to_string();
            PrivateKey::decode(key_str)
                .with_context(|| format!("failed to parse key from {file:?}"))
        } else {
            get_signing_key(&Some(registry_url.clone()), &self.key_name, config)
        }
    }
    /// Gets the auth token for the given registry URL.
    pub fn auth_token(&self, config: &Config) -> Result<Option<Secret<String>>> {
        if let Some(file) = &self.token_file {
            Ok(Some(Secret::from(
                std::fs::read_to_string(file)
                    .with_context(|| format!("failed to read key from {file:?}"))?
                    .trim_end()
                    .to_string(),
            )))
        } else {
            let tok = get_auth_token(
                &RegistryUrl::new(config.home_url.as_ref().unwrap())?,
                &self.token_name,
            )
            .map(Some)?;
            Ok(tok)
        }
    }
}

/// Namespace mapping to store when retrying a command after receiving a hint header
pub struct Retry {
    namespace: String,
    registry: String,
}

impl Retry {
    /// New Retry
    pub fn new(namespace: String, registry: String) -> Self {
        Self {
            namespace,
            registry,
        }
    }

    /// Map namespace using Retry information
    pub async fn store_namespace<R: RegistryStorage, C: ContentStorage, N: NamespaceMapStorage>(
        &self,
        client: &Client<R, C, N>,
    ) -> Result<()> {
        client
            .store_namespace(self.namespace.clone(), self.registry.clone())
            .await?;
        Ok(())
    }
}
