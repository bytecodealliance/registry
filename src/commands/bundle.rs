use super::CommonOptions;
use anyhow::Result;
use clap::Args;
use std::fs;
use warg_protocol::registry::PackageId;
use wasm_bundle::Bundler;

/// Bundle With Registry Dependencies
#[derive(Args)]
pub struct BundleCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// Only show information for the specified package.
    #[clap(value_name = "PACKAGE")]
    pub package: Option<PackageId>,
}

impl BundleCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        println!("registry: {url}", url = client.url());
        println!("\npackages in client storage:");
        let mut bundler = Bundler::new(&client);
        let locked = fs::read("./locked.wasm")?;
        let bundled = bundler.parse(&locked).await?;
        fs::write("./bundled.wasm", bundled.as_slice())?;
        Ok(())
    }
}
