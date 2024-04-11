use super::CommonOptions;
use anyhow::{bail, Result};
use clap::Args;
use semver::VersionReq;
use warg_client::storage::RegistryStorage;
use warg_client::Retry;
use warg_protocol::registry::PackageName;
/// Bundle With Registry Dependencies
#[derive(Args, Clone)]
pub struct BundleCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// Only show information for the specified package.
    #[clap(value_name = "PACKAGE")]
    pub package: PackageName,
}

impl BundleCommand {
    /// Executes the command.
    pub async fn exec(self, retry: Option<Retry>) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config, retry).await?;
        let registry_domain = client.get_warg_registry(self.package.namespace()).await?;
        println!("registry: {url}", url = client.url());
        if let Some(info) = client
            .registry()
            .load_package(registry_domain.as_ref(), &self.package)
            .await?
        {
            client.bundle_component(&info).await?;
        } else {
            client
                .download(registry_domain.as_ref(), &self.package, &VersionReq::STAR)
                .await?;
            if let Some(info) = client
                .registry()
                .load_package(registry_domain.as_ref(), &self.package)
                .await?
            {
                client.bundle_component(&info).await?;
            } else {
                bail!("Unable to find package {}", self.package.name())
            }
        }
        Ok(())
    }
}
