use super::CommonOptions;
use anyhow::{bail, Result};
use clap::Args;
use warg_client::storage::{ClientStorage, RegistryInfo};

/// Initializes a new warg registry.
#[derive(Args)]
pub struct InitCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The URL of the warg registry to use.
    #[clap(value_name = "REGISTRY")]
    pub registry: String,
}

impl InitCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        println!(
            "initializing registry at `{path}` with URL `{registry}`...",
            path = self.common.storage.display(),
            registry = self.registry
        );

        let storage = self.common.lock_storage()?;

        match storage.load_registry_info().await? {
            Some(_) => bail!(
                "registry at `{path}` has already been initialized",
                path = self.common.storage.display()
            ),
            None => {
                storage
                    .store_registry_info(&RegistryInfo::Remote { url: self.registry })
                    .await
            }
        }
    }
}
