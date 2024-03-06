//! Commands for the `warg` tool.

use anyhow::Result;
use clap::Args;
use secrecy::Secret;
use std::path::PathBuf;
use std::str::FromStr;
use warg_client::storage::ContentStorage;
use warg_client::storage::NamespaceMapStorage;
use warg_client::storage::RegistryDomain;
use warg_client::storage::RegistryStorage;
use warg_client::Client;
use warg_client::RegistryUrl;
use warg_client::{ClientError, Config, FileSystemClient, StorageLockResult};
use warg_credentials::keyring::{get_auth_token, get_signing_key};
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
    pub async fn create_client(
        &self,
        config: &Config,
        retry: Option<Retry>,
    ) -> Result<FileSystemClient, ClientError> {
        let client = match FileSystemClient::try_new_with_config(
            self.registry.as_deref(),
            config,
            self.auth_token(config)?,
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
                    self.auth_token(config)?.map(Secret::from),
                )
            }
        }?;
        if let Some(retry) = retry {
            retry.store_namespace(&client).await?;
        }
        Ok(client)
    }

    /// Gets the signing key for the given registry URL.
    pub fn signing_key<R: RegistryStorage, C: ContentStorage, N: NamespaceMapStorage>(
        &self,
        client: &Client<R, C, N>,
    ) -> Result<PrivateKey> {
        let registry_url = if let Some(nm) = &client.get_warg_registry() {
            Some(RegistryUrl::new(nm.to_string())?)
        } else {
            None
        };
        let config = self.read_config()?;
        get_signing_key(
            registry_url.map(|reg| reg.safe_label()).as_deref(),
            &config.keys.expect("Please set a default signing key by typing `warg key set <alg:base64>` or `warg key new"),
            config.home_url.as_deref(),
        )
    }
    /// Gets the auth token for the given registry URL.
    pub fn auth_token(&self, config: &Config) -> Result<Option<Secret<String>>> {
        if config.auth {
            return if let Some(reg_url) = &self.registry {
                Ok(get_auth_token(&RegistryUrl::new(reg_url)?)?)
            } else if let Some(url) = config.home_url.as_ref() {
                Ok(get_auth_token(&RegistryUrl::new(url)?)?)
            } else {
                Ok(None)
            };
        }
        Ok(None)
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
            .store_namespace(
                self.namespace.clone(),
                RegistryDomain::from_str(&self.registry)?,
            )
            .await?;
        Ok(())
    }
}
