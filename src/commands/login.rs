use anyhow::{bail, Context, Result};
use clap::Args;
use dialoguer::{theme::ColorfulTheme, Password};
use warg_client::RegistryUrl;

use crate::keyring::set_auth_token;

use super::CommonOptions;

/// Manage auth tokens for interacting with a registry.
#[derive(Args)]
pub struct LoginCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// The subcommand to execute.
    #[clap(flatten)]
    keyring_entry: KeyringEntryArgs,
}

#[derive(Args)]
struct KeyringEntryArgs {
    /// The URL of the registry to store an auth token for.
    #[clap(value_name = "URL")]
    pub url: Option<RegistryUrl>,
}

impl KeyringEntryArgs {
    fn set_entry(&self, home_url: Option<String>, token: &str) -> Result<()> {
        if let Some(url) = &self.url {
            set_auth_token(url, token)
        } else if let Some(url) = &home_url {
            set_auth_token(&RegistryUrl::new(url)?, token)
        } else {
            bail!("Please configure your home registry: warg config --registry <registry-url>")
        }
    }
}

impl LoginCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let token = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter auth token")
            .interact()
            .context("failed to read token")?;
        self.keyring_entry
            .set_entry(self.common.read_config()?.home_url, &token)?;
        println!("auth token was set successfully",);
        Ok(())
    }
}
