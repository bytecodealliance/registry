use anyhow::{bail, Result};
use clap::Args;
use warg_client::{keyring::Keyring, Config, RegistryUrl};

use super::CommonOptions;

/// Manage auth tokens for interacting with a registry.
#[derive(Args)]
pub struct LogoutCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// The URL of the registry to use.
    #[clap(value_name = "URL")]
    #[arg(hide = true)]
    pub registry_url: Option<String>,
}

impl LogoutCommand {
    /// Executes the command.
    pub async fn exec(mut self) -> Result<()> {
        if self.registry_url.is_some() {
            if self.common.registry.is_some() {
                bail!("Registry URL provided in two different arguments. Use only one.");
            }
            self.common.registry = self.registry_url;
        }
        let mut config = self.common.read_config()?;
        let registry_url = &self
            .common
            .registry
            .as_deref()
            .or(config.home_url.as_deref())
            .map(RegistryUrl::new)
            .transpose()?
            .ok_or(anyhow::anyhow!(
                "Registry is not specified, so nothing to logout."
            ))?;
        let keyring = Keyring::from_config(&config)?;
        keyring.delete_auth_token(registry_url)?;
        let registry_url_str = registry_url.to_string();
        if config
            .home_url
            .as_deref()
            .is_some_and(|home_url| home_url == registry_url_str)
        {
            config.keyring_auth = false;
            config.write_to_file(&Config::default_config_path()?)?;
        }
        println!(
            "Logged out of registry: {registry}",
            registry = registry_url.registry_domain(),
        );
        Ok(())
    }
}
