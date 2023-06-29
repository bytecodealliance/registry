use crate::keyring::{delete_signing_key, get_signing_key_entry, set_signing_key};
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm};
use keyring::Error as KeyringError;
use p256::ecdsa::SigningKey;
use rand_core::OsRng;
use warg_crypto::signing::PrivateKey;

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
    /// Sets the signing key for a registry in the local keyring.
    Set(KeySetCommand),
    /// Deletes the signing key for a registry from the local keyring.
    Delete(KeyDeleteCommand),
}

/// Creates a new signing key for a registry in the local keyring.
#[derive(Args)]
pub struct KeyNewCommand {
    /// The name to use for the signing key.
    #[clap(long, short, value_name = "KEY_NAME", default_value = "default")]
    pub key_name: String,
    /// The host name of the registry to create a signing key for.
    #[clap(value_name = "HOST")]
    pub host: String,
}

impl KeyNewCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let entry = get_signing_key_entry(&self.host, &self.key_name)?;

        match entry.get_password() {
            Err(KeyringError::NoEntry) => {
                // no entry exists, so we can continue
            }
            Ok(_) | Err(KeyringError::Ambiguous(_)) => {
                bail!(
                    "a signing key `{name}` already exists for registry `{host}`",
                    name = self.key_name,
                    host = self.host
                );
            }
            Err(e) => {
                bail!(
                    "failed to get signing key `{name}` for registry `{host}`: {e}",
                    name = self.key_name,
                    host = self.host
                );
            }
        }

        let key = SigningKey::random(&mut OsRng).into();
        set_signing_key(&self.host, &self.key_name, &key)?;

        Ok(())
    }
}

/// Sets the signing key for a registry in the local keyring.
#[derive(Args)]
pub struct KeySetCommand {
    /// The name to use for the signing key.
    #[clap(long, short, value_name = "KEY_NAME", default_value = "default")]
    pub key_name: String,
    /// The host name of the registry to set the signing key for.
    #[clap(value_name = "HOST")]
    pub host: String,
}

impl KeySetCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let key_str =
            rpassword::prompt_password("input signing key (expected format is `<alg>:<base64>`): ")
                .context("failed to read signing key")?;
        let key =
            PrivateKey::decode(key_str).context("signing key is not in the correct format")?;

        set_signing_key(&self.host, &self.key_name, &key)?;

        println!(
            "signing key `{name}` for registry `{host}` was set successfully",
            name = self.key_name,
            host = self.host,
        );

        Ok(())
    }
}

/// Deletes the signing key for a registry from the local keyring.
#[derive(Args)]
pub struct KeyDeleteCommand {
    /// The name to use for the signing key.
    #[clap(long, short, value_name = "KEY_NAME", default_value = "default")]
    pub key_name: String,
    /// The host name of the registry to delete the signing key for.
    #[clap(value_name = "HOST")]
    pub host: String,
}

impl KeyDeleteCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let prompt = format!(
            "are you sure you want to delete the signing key `{name}` for registry `{host}`? ",
            name = self.key_name,
            host = self.host
        );

        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .interact()?
        {
            delete_signing_key(&self.host, &self.key_name)?;
            println!(
                "signing key `{name}` for registry `{host}` was deleted successfully",
                name = self.key_name,
                host = self.host,
            );
        } else {
            println!(
                "skipping deletion of signing key for registry `{host}`",
                host = self.host,
            );
        }

        Ok(())
    }
}
