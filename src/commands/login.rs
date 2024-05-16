use crate::commands::config::{keyring_backend_help, keyring_backend_parser};
use anyhow::{bail, Context, Result};
use clap::Args;
use dialoguer::{theme::ColorfulTheme, Confirm, Password};
use p256::ecdsa::SigningKey;
use rand_core::OsRng;
use warg_client::keyring::Keyring;
use warg_client::{Config, RegistryUrl};

use super::CommonOptions;

/// Manage auth tokens for interacting with a registry.
#[derive(Args)]
pub struct LoginCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// Ignore federation hints.
    #[clap(long)]
    pub ignore_federation_hints: bool,

    /// Auto accept federation hints.
    #[clap(long)]
    pub auto_accept_federation_hints: bool,

    /// The backend to use for keyring access
    #[clap(long, value_name = "KEYRING_BACKEND", value_parser = keyring_backend_parser, long_help = keyring_backend_help())]
    pub keyring_backend: Option<String>,
}

impl LoginCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let mut config = self.common.read_config()?;
        let mut registry_url = &self
            .common
            .registry
            .as_ref()
            .map(RegistryUrl::new)
            .transpose()?
            .map(|u| u.to_string());
        config.ignore_federation_hints = self.ignore_federation_hints;
        config.auto_accept_federation_hints = self.auto_accept_federation_hints;

        // set keyring backend, if specified
        if self.keyring_backend.is_some() {
            config.keyring_backend = self.keyring_backend;
        }

        if registry_url.is_none() && config.home_url.is_none() {
            bail!("Please set your registry: warg login --registry <registry-url>");
        }

        let mut changing_home_registry = false;

        if registry_url.is_some()
            && registry_url != &config.home_url
            && Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!(
                    "Set `{registry}` as your home (or default) registry?",
                    registry = registry_url.as_deref().unwrap(),
                ))
                .default(true)
                .interact()?
        {
            config.home_url.clone_from(registry_url);
            config.write_to_file(&Config::default_config_path()?)?;

            // reset if changing home registry
            changing_home_registry = true;
        } else if registry_url.is_none() {
            registry_url = &config.home_url;
        }

        let keyring = Keyring::from_config(&config)?;
        config.keyring_auth = true;

        let client = if *registry_url == config.home_url {
            self.common.create_client(&config).await?
        } else {
            let mut config = config.clone();
            config.home_url.clone_from(registry_url);
            self.common.create_client(&config).await?
        };

        // the client may resolve the registry to well-known on a different registry host,
        // so replace the `registry_url` with that host
        let registry_url = Some(client.url().to_string());

        if changing_home_registry {
            client.reset_namespaces().await?;
            client.reset_registry().await?;
        }

        let prompt = format!(
            "Enter auth token for registry: {registry}",
            registry = client.url().registry_domain(),
        );

        if config.keys.is_empty() {
            config.keys.insert("default".to_string());
            let key = SigningKey::random(&mut OsRng).into();
            keyring.set_signing_key(None, &key, &mut config.keys, registry_url.as_deref())?;
            let public_key = key.public_key();
            let token = Password::with_theme(&ColorfulTheme::default())
                .with_prompt(prompt)
                .interact()
                .context("failed to read token")?;
            keyring.set_auth_token(&RegistryUrl::new(registry_url.as_deref().unwrap())?, &token)?;
            config.write_to_file(&Config::default_config_path()?)?;
            println!("Auth token was set successfully, and generated default key.");
            println!("Public Key: {public_key}");
            return Ok(());
        }

        let token = Password::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .interact()
            .context("failed to read token")?;
        keyring.set_auth_token(&RegistryUrl::new(registry_url.as_deref().unwrap())?, &token)?;
        config.write_to_file(&Config::default_config_path()?)?;
        println!("Auth token was set successfully.");

        if let Ok(private_key) = keyring.get_signing_key(
            self.common.registry.as_deref(),
            &config.keys,
            registry_url.as_deref(),
        ) {
            println!("\nSigning key is still available:");
            let public_key = private_key.public_key();
            println!("Key ID: {}", public_key.fingerprint());
            println!("Public Key: {public_key}");
        }

        Ok(())
    }
}
