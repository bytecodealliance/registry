use super::CommonOptions;
use anyhow::Result;
use clap::Args;
use warg_client::storage::{PackageInfo, RegistryStorage};
use warg_crypto::hash::AnyHash;
use warg_protocol::{registry::PackageId, Version};

/// Display client storage information.
#[derive(Args)]
pub struct InfoCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// Only show information for the specified package.
    #[clap(value_name = "PACKAGE")]
    pub package: Option<PackageId>,
}

impl InfoCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        println!("registry: {url}", url = client.url());
        println!("\npackages in client storage:");
        match self.package {
            Some(package) => {
                if let Some(info) = client.registry().load_package(&package).await? {
                    Self::print_package_info(&info);
                }
            }
            None => {
                client
                    .registry()
                    .load_packages()
                    .await?
                    .iter()
                    .for_each(Self::print_package_info);
            }
        }

        Ok(())
    }

    fn print_package_info(info: &PackageInfo) {
        println!("  id: {id}", id = info.id);
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
