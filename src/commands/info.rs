use super::CommonOptions;
use anyhow::{anyhow, Result};
use clap::Args;
use warg_client::storage::{ClientStorage, PackageInfo, RegistryInfo};
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

        if let Some(url) = registry_info.url() {
            println!("registry: {url}", url = url);
        }

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

        if let RegistryInfo::Local { packages, origins } = &registry_info {
            println!("\nlocal packages in client storage:");
            for (name, versions) in packages {
                println!("  name: {name}", name = name);
                if let Some(origin) = origins.get(name) {
                    println!("  origin: {origin}");
                }
                println!("  versions:");
                for (version, content) in versions {
                    Self::print_release(version, content);
                }
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
