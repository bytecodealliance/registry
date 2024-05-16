use super::CommonOptions;
use anyhow::Result;
use clap::Args;

/// Update all local package logs.
#[derive(Args)]
pub struct UpdateCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl UpdateCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config).await?;

        println!("updating package logs to the latest available versions...");
        client.update().await?;

        Ok(())
    }
}
