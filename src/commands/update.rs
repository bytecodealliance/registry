use super::CommonOptions;
use anyhow::Result;
use clap::Args;

/// Update all local packages in the registry.
#[derive(Args)]
pub struct UpdateCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl UpdateCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        println!("updating packages to the latest available versions...");
        let mut client = self.common.create_client()?;
        client.update().await?;
        Ok(())
    }
}
