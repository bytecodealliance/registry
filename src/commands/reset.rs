use super::CommonOptions;
use anyhow::Result;
use clap::Args;

/// Reset local data for registry.
#[derive(Args)]
pub struct ResetCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl ResetCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config).await?;

        println!("resetting local registry data...");
        client.reset_registry().await?;
        client.reset_namespaces().await?;

        Ok(())
    }
}
