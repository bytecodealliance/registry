use super::CommonOptions;
use anyhow::Result;
use clap::{ArgAction, Args};

/// Update all local package logs for a registry.
#[derive(Args, Clone)]
pub struct UpdateCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// The common command options.
    #[clap(short, long, value_name = "ALL", action = ArgAction::SetTrue)]
    pub all: bool,
}

impl UpdateCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let mut client = self.common.create_client(&config).await?;

        println!("updating package logs to the latest available versions...");
        if self.all {
            client.update_all().await?;
        } else {
            client.update().await?;
        }

        Ok(())
    }
}
