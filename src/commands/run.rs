use super::CommonOptions;
use crate::demo;
use anyhow::Result;
use clap::Args;
use warg_client::FileSystemStorage;

/// Run a package.
#[derive(Args)]
pub struct RunCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The name of the package to run.
    pub name: String,
    /// The arguments to the package.
    pub args: Vec<String>,
}

impl RunCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        println!("running package `{name}`", name = self.name);

        let storage = FileSystemStorage::new(&self.common.storage)?;
        let client = self.common.create_client()?;
        let content_digest = client.get_latest_version(&self.name).await?;
        let path = storage.content_path(&content_digest)?;
        demo::run_wasm(path, &self.args)
    }
}
