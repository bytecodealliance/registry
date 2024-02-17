use super::{CommonOptions, Retry};
use anyhow::Result;
use clap::Args;
use semver::VersionReq;
use warg_client::{
    storage::{PackageInfo, RegistryStorage},
    FileSystemClient,
};
use warg_protocol::registry::PackageName;

/// Print Dependency Tree
#[derive(Args)]
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
    pub async fn exec(self, retry: Option<Retry>) -> Result<()> {
        let config = self.common.read_config()?;
        let mut client = self.common.create_client(&config)?;
        let auth_token = self.common.auth_token()?;
        if let Some(retry) = retry {
            retry.store_namespace(&client).await?
        }
        client
            .refresh_namespace(&auth_token, self.package.namespace())
            .await?;
        println!("registry: {url}", url = client.url());
        if let Some(info) = client
            .registry()
            .load_package(client.get_warg_header(), &self.package)
            .await?
        {
            Self::lock(&auth_token, client, &info).await?;
        } else {
            client
                .download(&auth_token, &self.package, &VersionReq::STAR)
                .await?;
            if let Some(info) = client
                .registry()
                .load_package(client.get_warg_header(), &self.package)
                .await?
            {
                Self::lock(&auth_token, client, &info).await?;
            }
        }
        Ok(())
    }

    async fn lock(
        auth_token: &Option<String>,
        client: FileSystemClient,
        info: &PackageInfo,
    ) -> Result<()> {
        client.lock_component(&auth_token, info).await?;
        Ok(())
    }
}
