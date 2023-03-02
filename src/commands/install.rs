use super::CommonOptions;
use anyhow::Result;
use clap::Args;

/// Install a warg registry package.
#[derive(Args)]
pub struct InstallCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The name of the package to install.
    #[clap(value_name = "PACKAGE")]
    pub package: String,
}

impl InstallCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        println!("installing package `{package}`...", package = self.package);
        let mut client = self.common.create_client()?;
        client.install(self.package).await?;
        Ok(())
    }
}
