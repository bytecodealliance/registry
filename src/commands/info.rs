use super::CommonOptions;
use anyhow::Result;
use clap::{ArgAction, Args};
use warg_client::{
    storage::{ContentStorage, NamespaceMapStorage, PackageInfo, RegistryStorage},
    Client,
};
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

    /// Only show the namespace map
    #[clap(short, long, value_name = "NAMESPACES", action = ArgAction::SetTrue)]
    pub namespaces: bool,
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
                let info = client.package(&package).await?;
                if let Some(registry) = client.get_warg_registry(package.namespace()).await? {
                    println!("registry: {registry}");
                }
                Self::print_package_info(&info);
            }
            None => {
                client
                    .registry()
                    .load_all_packages()
                    .await?
                    .iter()
                    .for_each(|(registry, packages)| {
                        println!("registry: {registry}");
                        packages.iter().for_each(Self::print_package_info);
                    });
            }
        }

        if self.namespaces {
            println!("\nnamespace mappings in client storage");
            Self::print_namespace_map(&client).await?;
            return Ok(());
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

    async fn print_namespace_map<R: RegistryStorage, C: ContentStorage, N: NamespaceMapStorage>(
        client: &Client<R, C, N>,
    ) -> Result<()> {
        if let Some(map) = client.namespace_map().load_namespace_map().await? {
            for (namespace, registry) in map {
                println!("  {namespace}={registry}");
            }
        };

        Ok(())
    }
}
