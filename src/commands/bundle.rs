use super::CommonOptions;
use anyhow::Result;
use async_recursion::async_recursion;
use clap::Args;
use ptree::{output::print_tree, TreeBuilder};
use reqwest;
use std::collections::HashMap;
use std::fs;
use warg_api::v1::fetch::FetchDependenciesResponse;
use warg_client::{
    storage::{PackageInfo, RegistryStorage},
    FileSystemClient,
};
use warg_crypto::hash::{Sha256};
use warg_protocol::{
    registry::{LogId, PackageId},
};
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
