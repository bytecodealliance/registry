use super::CommonOptions;
use anyhow::{bail, Result};
use clap::Args;
use warg_client::RegistryInfo;

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

        let mut client = self.common.create_client()?;

        match client.storage().load_registry_info().await? {
            Some(_) => bail!("registry has already been initialized"),
            None => {
                client
                    .storage()
                    .store_registry_info(&RegistryInfo {
                        url: self.registry,
                        checkpoint: None,
                    })
                    .await
            }
        }
    }
}
