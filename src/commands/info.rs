use super::CommonOptions;
use anyhow::{anyhow, Result};
use clap::Args;
use warg_client::storage::{ClientStorage, PackageInfo};
use warg_crypto::hash::DynHash;
use warg_protocol::Version;

/// Display client storage information.
#[derive(Args)]
pub struct InfoCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// Only show information for the specified package.
    #[clap(value_name = "PACKAGE")]
    pub package: Option<String>,
}

impl InfoCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let storage = self.common.lock_storage()?;

        let registry_info = storage
            .load_registry_info()
            .await?
            .ok_or_else(|| anyhow!("the registry is not initialized"))?;

        println!("registry: {url}", url = registry_info.url);
        println!("\npackages in client storage:");
        match self.package {
            Some(package) => {
                if let Some(info) = storage.load_package_info(&package).await? {
                    Self::print_package_info(&info);
                }
            }
            None => {
                storage
                    .load_packages()
                    .await?
                    .iter()
                    .for_each(Self::print_package_info);
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

    fn print_release(version: &Version, content: &DynHash) {
        println!("    {version} ({content})");
    }
}
