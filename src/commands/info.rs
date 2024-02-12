use super::CommonOptions;
use anyhow::Result;
use clap::Args;
use warg_client::storage::{PackageInfo, RegistryStorage};
use warg_crypto::hash::AnyHash;
use warg_protocol::{registry::PackageName, Version};

/// Display client storage information.
#[derive(Args)]
pub struct InfoCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// Only show information for the specified package.
    #[clap(value_name = "PACKAGE")]
    pub package: Option<PackageName>,
}

impl InfoCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let mut client = self.common.create_client(&config)?;

        println!("registry: {url}", url = client.url());
        println!("\npackages in client storage:");
        match self.package {
            Some(package) => {
                client.fetch_well_known().await?;
                if let Some(info) = client
                    .registry()
                    .load_package(client.namespace_registry(), client.well_known(), &package)
                    .await?
                {
                    Self::print_package_info(&info);
                }
            }
            None => {
                client
                    .registry()
                    .load_all_packages()
                    .await?
                    .iter()
                    .for_each(|(_, packages)| packages.iter().for_each(Self::print_package_info));
            }
        }

        Ok(())
    }

    fn print_package_info(info: &PackageInfo) {
        println!("  name: {name}", name = info.name);
        println!("  versions:");
        info.state.releases().for_each(|r| {
            if let Some(content) = r.content() {
                Self::print_release(&r.version, content);
            }
        });
    }

    fn print_release(version: &Version, content: &AnyHash) {
        println!("    {version} ({content})");
    }
}
