use super::CommonOptions;
use anyhow::Result;
use clap::Args;

/// Update all local package logs for a registry.
#[derive(Args)]
pub struct UpdateCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// Update package logs for all registries in storage
    pub all: Option<bool>,
}

impl UpdateCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let mut client = self.common.create_client(&config)?;
        client.fetch_well_known().await?;

        println!("updating package logs to the latest available versions...");
        client.update().await?;
        Ok(())
    }
}
