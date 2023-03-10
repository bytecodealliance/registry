//! Commands for the `warg-cli` tool.

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;
use warg_client::storage::FileSystemStorage;
use warg_client::Client;

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
        FileSystemStorage::lock(&self.storage, |path| {
            println!("Blocking on registry lock file `{}`", path.display());
            Ok(())
        })
    }

    /// Creates the warg client to use.
    pub async fn create_client(&self) -> Result<Client<FileSystemStorage>> {
        Ok(Client::new(self.lock_storage()?).await?)
    }
}
