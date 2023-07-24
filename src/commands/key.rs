use crate::keyring::{delete_signing_key, get_signing_key, get_signing_key_entry, set_signing_key};
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm};
use keyring::{Entry, Error as KeyringError};
use p256::ecdsa::SigningKey;
use rand_core::OsRng;
use warg_client::RegistryUrl;
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
            KeySubcommand::Id(cmd) => cmd.exec().await,
        }
    }
}

/// The subcommand to execute.
#[derive(Subcommand)]
pub enum KeySubcommand {
    /// Creates a new signing key for a registry in the local keyring.
    New(KeyNewCommand),
    /// Shows the ID of the signing key for a registry in the local keyring.
    Id(KeyIdCommand),
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
    pub url: RegistryUrl,
}

impl KeyringEntryArgs {
    fn get_entry(&self) -> Result<Entry> {
        get_signing_key_entry(&self.url, &self.name)
    }

    fn get_key(&self) -> Result<PrivateKey> {
        get_signing_key(&self.url, &self.name)
    }

    fn set_entry(&self, key: &PrivateKey) -> Result<()> {
        set_signing_key(&self.url, &self.name, key)
    }

    fn delete_entry(&self) -> Result<()> {
        delete_signing_key(&self.url, &self.name)
    }
}

impl std::fmt::Display for KeyringEntryArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "`{name}` for registry `{url}`",
            name = self.name,
            url = self.url
        )
    }
}

/// Creates a new signing key for a registry in the local keyring.
#[derive(Args)]
pub struct KeyNewCommand {
    #[clap(flatten)]
    keyring_entry: KeyringEntryArgs,
}

impl KeyNewCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let entry = self.keyring_entry.get_entry()?;

        match entry.get_password() {
            Err(KeyringError::NoEntry) => {
                // no entry exists, so we can continue
            }
            Ok(_) | Err(KeyringError::Ambiguous(_)) => {
                bail!(
                    "a signing key `{name}` already exists for registry `{url}`",
                    name = self.keyring_entry.name,
                    url = self.keyring_entry.url
                );
            }
            Err(e) => {
                bail!(
                    "failed to get signing key {entry}: {e}",
                    entry = self.keyring_entry
                );
            }
        }

        let key = SigningKey::random(&mut OsRng).into();
        self.keyring_entry.set_entry(&key)?;

        Ok(())
    }
}

/// Shows the ID of the signing key for a registry in the local keyring.
#[derive(Args)]
pub struct KeyIdCommand {
    #[clap(flatten)]
    keyring_entry: KeyringEntryArgs,
}

impl KeyIdCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let private_key = self.keyring_entry.get_key()?;
        println!("{}", private_key.public_key().fingerprint());
        Ok(())
    }
}

/// Sets the signing key for a registry in the local keyring.
#[derive(Args)]
pub struct KeySetCommand {
    #[clap(flatten)]
    keyring_entry: KeyringEntryArgs,
}

impl KeySetCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let key_str =
            rpassword::prompt_password("input signing key (expected format is `<alg>:<base64>`): ")
                .context("failed to read signing key")?;
        let key =
            PrivateKey::decode(key_str).context("signing key is not in the correct format")?;

        self.keyring_entry.set_entry(&key)?;

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
}

impl KeyDeleteCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let prompt = format!(
            "are you sure you want to delete the signing key {entry}? ",
            entry = self.keyring_entry
        );

        if Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(prompt)
            .interact()?
        {
            self.keyring_entry.delete_entry()?;
            println!(
                "signing key {entry} was deleted successfully",
                entry = self.keyring_entry
            );
        } else {
            println!(
                "skipping deletion of signing key for registry `{url}`",
                url = self.keyring_entry.url,
            );
        }

        Ok(())
    }
}
