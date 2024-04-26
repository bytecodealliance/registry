use super::CommonOptions;
use anyhow::Result;
use clap::Args;
use semver::VersionReq;
use warg_client::{
    storage::{PackageInfo, RegistryStorage},
    FileSystemClient,
};
use warg_protocol::registry::PackageName;

/// Print Dependency Tree
#[derive(Args, Clone)]
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
        let client = self.common.create_client(&config).await?;
        let registry_domain = client.get_warg_registry(self.package.namespace()).await?;
        println!("registry: {url}", url = client.url());
        if let Some(info) = client
            .registry()
            .load_package(registry_domain.as_ref(), &self.package)
            .await?
        {
            Self::lock(client, &info).await?;
        } else {
            client
                .download(registry_domain.as_ref(), &self.package, &VersionReq::STAR)
                .await?;
            if let Some(info) = client
                .registry()
                .load_package(
                    client
                        .get_warg_registry(self.package.namespace())
                        .await?
                        .as_ref(),
                    &self.package,
                )
                .await?
            {
                Self::lock(client, &info).await?;
            }
        }
        Ok(())
    }

    async fn lock(client: FileSystemClient, info: &PackageInfo) -> Result<()> {
        client.lock_component(info).await?;
        Ok(())
    }
}
