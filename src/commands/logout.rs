use anyhow::{bail, Result};
use clap::Args;
use warg_client::{Config, RegistryUrl};
use warg_credentials::keyring::Keyring;

use super::CommonOptions;

/// Manage auth tokens for interacting with a registry.
#[derive(Args)]
pub struct LogoutCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The subcommand to execute.
    #[clap(flatten)]
    keyring_entry: KeyringEntryArgs,
}

#[derive(Args)]
struct KeyringEntryArgs {
    /// The URL of the registry to delete an auth token for.
    #[clap(value_name = "URL")]
    pub url: Option<RegistryUrl>,
}

impl KeyringEntryArgs {
    fn delete_entry(&self, keyring: &Keyring, home_url: Option<String>) -> Result<()> {
        if let Some(url) = &self.url {
            keyring.delete_auth_token(url)?;
        } else if let Some(url) = &home_url {
            keyring.delete_auth_token(&RegistryUrl::new(url)?)?;
        } else {
            bail!("Please configure your home registry: warg config --registry <registry-url>")
        }
        Ok(())
    }
}

impl LogoutCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let mut config = self.common.read_config()?;
        let keyring = Keyring::from_config(&config)?;
        self.keyring_entry
            .delete_entry(&keyring, config.home_url.clone())?;
        config.keyring_auth = false;
        config.write_to_file(&Config::default_config_path()?)?;
        println!("auth token was deleted successfully",);
        Ok(())
    }
}
