use super::CommonOptions;
use anyhow::Result;
use clap::Args;

/// Display information about registry packages.
#[derive(Args)]
pub struct InfoCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// The name of the package to inspect.
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
