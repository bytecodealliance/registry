use super::CommonOptions;
use anyhow::Result;
use clap::Args;

/// Reset local data for registry.
#[derive(Args)]
pub struct ResetCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// Whether to reset all registries.
    #[clap(long)]
    pub all: bool,
}

impl ResetCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        if self.all {
            println!("resetting local data for all registries...");
            client.reset_registry(true).await?;
        } else {
            println!("resetting local data for registry `{}`...", client.url());
            client.reset_registry(false).await?;
        }

        Ok(())
    }
}
