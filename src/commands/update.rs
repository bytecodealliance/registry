use super::CommonOptions;
use anyhow::Result;
use clap::{ArgAction, Args};

/// Update all local package logs for a registry.
#[derive(Args)]
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
        let mut client = self.common.create_client(&config)?;
        let auth_token = self.common.auth_token()?;

        println!("updating package logs to the latest available versions...");
        if self.all {
            client.update_all(&auth_token).await?;
        } else {
            client.update(&auth_token).await?;
        }

        Ok(())
    }
}
