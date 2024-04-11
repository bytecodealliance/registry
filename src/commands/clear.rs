use super::CommonOptions;
use anyhow::Result;
use clap::Args;

/// Deletes local content cache.
#[derive(Args, Clone)]
pub struct ClearCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
}

impl ClearCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config, None).await?;

        println!("clearing local content cache...");
        client.clear_content_cache().await?;
        Ok(())
    }
}
