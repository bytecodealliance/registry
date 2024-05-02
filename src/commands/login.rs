use anyhow::{bail, Context, Result};
use clap::Args;
use dialoguer::{theme::ColorfulTheme, Password};
use p256::ecdsa::SigningKey;
use rand_core::OsRng;
use warg_client::{Config, RegistryUrl};

use warg_credentials::keyring::{set_auth_token, set_signing_key};

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

    /// Ignore federation hints.
    #[clap(long)]
    pub ignore_federation_hints: bool,

    /// Auto accept federation hints.
    #[clap(long)]
    pub auto_accept_federation_hints: bool,
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
        let home_url = &self
            .common
            .registry
            .clone()
            .map(RegistryUrl::new)
            .transpose()?
            .map(|u| u.to_string());
        let mut config = self.common.read_config()?;
        config.ignore_federation_hints = self.ignore_federation_hints;
        config.auto_accept_federation_hints = self.auto_accept_federation_hints;

        if home_url.is_some() {
            config.home_url = home_url.clone();
            config.write_to_file(&Config::default_config_path()?)?;

            // reset if changing home registry
            let client = self.common.create_client(&config)?;
            client.reset_namespaces().await?;
            client.reset_registry().await?;
        }

        config.keyring_auth = true;

        if config.keys.is_empty() {
            config.keys.insert("default".to_string());
            let key = SigningKey::random(&mut OsRng).into();
            set_signing_key(None, &key, &mut config.keys, config.home_url.as_deref())?;
            let public_key = key.public_key();
            let token = Password::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter auth token")
                .interact()
                .context("failed to read token")?;
            self.keyring_entry
                .set_entry(self.common.read_config()?.home_url, &token)?;
            config.write_to_file(&Config::default_config_path()?)?;
            println!("auth token was set successfully, and generated default key",);
            println!("Public Key: {public_key}");
            return Ok(());
        }

        let token = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("Enter auth token")
            .interact()
            .context("failed to read token")?;
        self.keyring_entry
            .set_entry(self.common.read_config()?.home_url, &token)?;
        config.write_to_file(&Config::default_config_path()?)?;
        println!("auth token was set successfully",);
        Ok(())
    }
}
