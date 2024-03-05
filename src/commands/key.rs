use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm, Password};
use indexmap::IndexSet;
use p256::ecdsa::SigningKey;
use rand_core::OsRng;
use warg_client::Config;
use warg_credentials::keyring::{delete_signing_key, get_signing_key, set_signing_key};
use warg_crypto::signing::PrivateKey;

use super::CommonOptions;

/// Manage signing keys for interacting with a registry.
#[derive(Args)]
pub struct KeyCommand {
    /// The subcommand to execute.
    #[clap(subcommand)]
    pub command: KeySubcommand,
}

impl KeyCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        match self.command {
            KeySubcommand::New(cmd) => cmd.exec().await,
            KeySubcommand::Info(cmd) => cmd.exec().await,
            KeySubcommand::Set(cmd) => cmd.exec().await,
            KeySubcommand::Delete(cmd) => cmd.exec().await,
        }
    }
}

/// The subcommand to execute.
#[derive(Subcommand)]
pub enum KeySubcommand {
    /// Creates a new signing key for a registry in the local keyring.
    New(KeyNewCommand),
    /// Shows information about the signing key for a registry in the local keyring.
    Info(KeyInfoCommand),
    /// Sets the signing key for a registry in the local keyring.
    Set(KeySetCommand),
    /// Deletes the signing key for a registry from the local keyring.
    Delete(KeyDeleteCommand),
}

/// Creates a new signing key for a registry in the local keyring.
#[derive(Args)]
pub struct KeyNewCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl KeyNewCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = &mut self.common.read_config()?;
        let key = SigningKey::random(&mut OsRng).into();
        if config.keys.is_some() {
            if let Some(ref reg) = self.common.registry {
                config.keys.as_mut().unwrap().insert(reg.to_string());
            } else {
                config.keys.as_mut().unwrap().insert("default".to_string());
            }
        } else {
            let mut keys = IndexSet::new();
            keys.insert("default".to_string());
            config.keys = Some(keys);
        };
        set_signing_key(
            self.common.registry.as_deref(),
            &key,
            config.keys.as_mut().unwrap(),
            config.home_url.as_deref(),
        )?;
        config.write_to_file(&Config::default_config_path()?)?;
        let public_key = key.public_key();
        println!("Key ID: {}", public_key.fingerprint());
        println!("Public Key: {public_key}");
        Ok(())
    }
}

/// Shows information about the signing key for a registry in the local keyring.
#[derive(Args)]
pub struct KeyInfoCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl KeyInfoCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = &self.common.read_config()?;
        if let Some(keys) = &config.keys {
            let private_key = get_signing_key(
                self.common.registry.as_deref(),
                keys,
                config.home_url.as_deref(),
            )?;
            let public_key = private_key.public_key();
            println!("Key ID: {}", public_key.fingerprint());
            println!("Public Key: {public_key}");
            Ok(())
        } else {
            bail!("error: Please set a default signing key by typing `warg key set <alg:base64>` or `warg key new`")
        }
    }
}

/// Sets the signing key for a registry in the local keyring.
#[derive(Args)]
pub struct KeySetCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl KeySetCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let key_str = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("input signing key (expected format is `<alg>:<base64>`): ")
            .interact()
            .context("failed to read signing key")?;
        let key =
            PrivateKey::decode(key_str).context("signing key is not in the correct format")?;
        let config = &mut self.common.read_config()?;

        if config.keys.is_none() {
            config.keys = Some(IndexSet::new());
        }
        set_signing_key(
            self.common.registry.as_deref(),
            &key,
            config.keys.as_mut().unwrap(),
            config.home_url.as_deref(),
        )?;
        config.write_to_file(&Config::default_config_path()?)?;

        println!("signing key was set successfully");

        Ok(())
    }
}

/// Deletes the signing key for a registry from the local keyring.
#[derive(Args)]
pub struct KeyDeleteCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl KeyDeleteCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = &mut self.common.read_config()?;

        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("are you sure you want to delete your signing key")
            .interact()?
        {
            delete_signing_key(self.common.registry.as_deref(), config.keys.as_ref().expect("Please set a default signing key by typing `warg key set <alg:base64>` or `warg key new"), config.home_url.as_deref())?;
            let keys = &mut config.keys;
            if let Some(keys) = keys {
                if let Some(registry_url) = self.common.registry {
                    keys.swap_remove(&registry_url);
                } else {
                    keys.swap_remove("default");
                }
            }
            config.write_to_file(&Config::default_config_path()?)?;
            println!("signing key was deleted successfully",);
        } else if let Some(url) = self.common.registry {
            println!(
                "skipping deletion of signing key for registry `{url}`",
                url = url
            );
        } else {
            println!("skipping deletion of signing key");
        }

        Ok(())
    }
}
