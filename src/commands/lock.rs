use super::CommonOptions;
use anyhow::Result;
use clap::Args;
use warg_protocol::registry::PackageName;

/// Print Dependency Tree
#[derive(Args)]
pub struct LockCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// Only show information for the specified package.
    #[clap(value_name = "PACKAGE")]
    pub package: PackageName,
}

impl LockCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        let info = client.package(&self.package).await?;
        client.lock_component(&info).await?;

        Ok(())
    }
}
