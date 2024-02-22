use crate::keyring::{delete_signing_key, get_signing_key, get_signing_key_entry, set_signing_key};
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm, Password};
use keyring::{Entry, Error as KeyringError};
use p256::ecdsa::SigningKey;
use rand_core::OsRng;
use warg_client::{Config, RegistryUrl};
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

#[derive(Args)]
struct KeyringEntryArgs {
    /// The name to use for the signing key.
    #[clap(long, short, value_name = "KEY_NAME", default_value = "default")]
    pub name: String,
    /// The URL of the registry to create a signing key for.
    #[clap(value_name = "URL")]
    pub url: Option<RegistryUrl>,
}

impl KeyringEntryArgs {
    fn get_entry(&self, config: &Config) -> Result<Entry> {
        get_signing_key_entry(&self.url, &self.name, config)
    }

    fn get_key(&self, config: &Config) -> Result<PrivateKey> {
        get_signing_key(&self.url, &self.name, config)
    }

    fn set_entry(&self, key: &PrivateKey, config: &mut Config) -> Result<()> {
        set_signing_key(&self.url, &self.name, key, config)
    }

    fn delete_entry(&self, config: &Config) -> Result<()> {
        delete_signing_key(&self.url, &self.name, config)
    }
}

impl std::fmt::Display for KeyringEntryArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(url) = &self.url {
            write!(f, "`{name}` for registry `{url}`", name = self.name,)
        } else {
            write!(f, "{name}", name = self.name)
        }
    }
}

/// Creates a new signing key for a registry in the local keyring.
#[derive(Args)]
pub struct KeyNewCommand {
    #[clap(flatten)]
    keyring_entry: KeyringEntryArgs,
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl KeyNewCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = &mut self.common.read_config()?;

        let entry = self.keyring_entry.get_entry(config)?;

        match entry.get_password() {
            Err(KeyringError::NoEntry) => {
                // no entry exists, so we can continue
            }
            Ok(_) | Err(KeyringError::Ambiguous(_)) => {
                if let Some(url) = self.keyring_entry.url {
                    bail!(
                        "a signing key `{name}` already exists for registry `{url}`",
                        name = self.keyring_entry.name,
                    );
                } else {
                    bail!(
                        "a signing key `{name}` already exists",
                        name = self.keyring_entry.name,
                    );
                }
            }
            Err(e) => {
                bail!(
                    "failed to get signing key {entry}: {e}",
                    entry = self.keyring_entry
                );
            }
        }

        let key = SigningKey::random(&mut OsRng).into();
        self.keyring_entry.set_entry(&key, config)?;

        Ok(())
    }
}

/// Shows information about the signing key for a registry in the local keyring.
#[derive(Args)]
pub struct KeyInfoCommand {
    #[clap(flatten)]
    keyring_entry: KeyringEntryArgs,
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl KeyInfoCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = &self.common.read_config()?;
        let private_key = self.keyring_entry.get_key(config)?;
        let public_key = private_key.public_key();
        println!("Key ID: {}", public_key.fingerprint());
        println!("Public Key: {public_key}");
        Ok(())
    }
}

/// Sets the signing key for a registry in the local keyring.
#[derive(Args)]
pub struct KeySetCommand {
    #[clap(flatten)]
    keyring_entry: KeyringEntryArgs,
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

        self.keyring_entry.set_entry(&key, config)?;

        println!(
            "signing key {keyring} was set successfully",
            keyring = self.keyring_entry
        );

        Ok(())
    }
}

/// Deletes the signing key for a registry from the local keyring.
#[derive(Args)]
pub struct KeyDeleteCommand {
    #[clap(flatten)]
    keyring_entry: KeyringEntryArgs,
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl KeyDeleteCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = &self.common.read_config()?;
        let prompt = format!(
            "are you sure you want to delete the signing key {entry}? ",
            entry = self.keyring_entry
        );

        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .interact()?
        {
            self.keyring_entry.delete_entry(&config)?;
            println!(
                "signing key {entry} was deleted successfully",
                entry = self.keyring_entry
            );
        } else if let Some(url) = self.keyring_entry.url {
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
