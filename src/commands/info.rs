use super::CommonOptions;
use anyhow::Result;
use clap::Args;

/// Update all local packages in the registry.
#[derive(Args)]
pub struct InfoCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// The name of the package to install.
    #[clap(value_name = "PACKAGE")]
    pub package: String,
}

impl InfoCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let mut client = self.common.create_client()?;
        client.inform(self.package).await?;
        Ok(())
    }
}
