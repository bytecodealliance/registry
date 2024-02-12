use super::CommonOptions;
use anyhow::{anyhow, Result};
use clap::Args;
use warg_protocol::{registry::PackageName, VersionReq};

/// Download a warg registry package.
#[derive(Args)]
#[clap(disable_version_flag = true)]
pub struct DownloadCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The package name to download.
    #[clap(value_name = "PACKAGE")]
    pub name: PackageName,
    #[clap(long, short, value_name = "VERSION")]
    /// The version requirement of the package to download; defaults to `*`.
    pub version: Option<VersionReq>,
}

impl DownloadCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let mut client = self.common.create_client(&config)?;
        client.fetch_well_known().await?;
        client.map_namespace(self.name.namespace()).await;
        println!("downloading package `{name}`...", name = self.name);

        let res = client
            .download(
                &self.name,
                self.version.as_ref().unwrap_or(&VersionReq::STAR),
            )
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "a version of package `{name}` that satisfies `{version}` was not found",
                    name = self.name,
                    version = self.version.as_ref().unwrap_or(&VersionReq::STAR)
                )
            })?;

        println!(
            "downloaded version {version} of package `{name}` ({digest})",
            name = self.name,
            version = res.version,
            digest = res.digest
        );

        Ok(())
    }
}
