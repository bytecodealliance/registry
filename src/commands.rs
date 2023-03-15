//! Commands for the `warg-cli` tool.

use anyhow::{bail, Context, Result};
use clap::Args;
use keyring::Entry;
use std::path::PathBuf;
use warg_client::storage::FileSystemStorage;
use warg_client::Client;
use warg_crypto::signing;

mod download;
mod info;
mod init;
mod publish;
mod run;
mod update;

pub use self::download::*;
pub use self::info::*;
pub use self::init::*;
pub use self::publish::*;
pub use self::run::*;
pub use self::update::*;

const DEFAULT_STORAGE_PATH: &str = ".warg";

/// Common options for commands.
#[derive(Args)]
pub struct CommonOptions {
    /// The path to the registry storage directory to use.
    #[clap(long, value_name = "PATH", default_value = DEFAULT_STORAGE_PATH)]
    pub storage: PathBuf,
}

impl CommonOptions {
    /// Locks the client storage.
    pub fn lock_storage(&self) -> Result<FileSystemStorage> {
        match FileSystemStorage::try_lock(&self.storage)? {
            Some(storage) => Ok(storage),
            None => {
                println!(
                    "blocking on lock for registry `{path}`...",
                    path = self.storage.display()
                );
                Ok(FileSystemStorage::lock(&self.storage)?)
            }
        }
    }

    /// Creates the warg client to use.
    pub async fn create_client(&self) -> Result<Client<FileSystemStorage>> {
        Ok(Client::new(self.lock_storage()?).await?)
    }
}

fn get_signing_key() -> Result<signing::PrivateKey> {
    const WARG_SIGNING_KEY_SERVICE: &str = "warg-signing-key:registry.bytecodealliance.org";
    const WARG_KEYRING_USER: &str = "default";

    let desc = format!("{WARG_KEYRING_USER}@{WARG_SIGNING_KEY_SERVICE}");
    let ring = Entry::new(WARG_SIGNING_KEY_SERVICE, WARG_KEYRING_USER)?;

    match ring.get_password() {
        Ok(secret) => secret.parse().context("failed to parse signing key"),
        Err(keyring::Error::NoEntry) => {
            bail!("No credentials found for '{desc}')");
        }
        Err(keyring::Error::Ambiguous(creds)) => {
            bail!("More than one credential found for {desc}: {creds:?}")
        }
        Err(err) => {
            bail!("Couldn't get credentials for '{desc}': {err}",);
        }
    }
}
