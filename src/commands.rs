//! Commands for the `warg-cli` tool.

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;
use warg_client::Client;
use warg_client::FileSystemStorage;

mod init;
mod install;
mod publish;
mod run;
mod update;

pub use self::init::*;
pub use self::install::*;
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
    /// Creates the warg client to use.
    pub fn create_client(self) -> Result<Client> {
        let storage = FileSystemStorage::new(self.storage)?;
        Ok(Client::new(Box::new(storage)))
    }
}
