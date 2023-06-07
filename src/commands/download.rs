use super::CommonOptions;
use anyhow::{anyhow, Result};
use clap::Args;
use warg_protocol::{registry::PackageId, VersionReq};

/// Download a warg registry package.
#[derive(Args)]
#[clap(disable_version_flag = true)]
pub struct DownloadCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The identifier of the package to download.
    #[clap(value_name = "PACKAGE")]
    pub id: PackageId,
    #[clap(long, short, value_name = "VERSION")]
    /// The version requirement of the package to download; defaults to `*`.
    pub version: Option<VersionReq>,
}

impl DownloadCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        println!("downloading package `{id}`...", id = self.id);

        let res = client
            .download(&self.id, self.version.as_ref().unwrap_or(&VersionReq::STAR))
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "a version of package `{id}` that satisfies `{version}` was not found",
                    id = self.id,
                    version = self.version.as_ref().unwrap_or(&VersionReq::STAR)
                )
            })?;

        println!(
            "downloaded version {version} of package `{id}` ({digest})",
            id = self.id,
            version = res.version,
            digest = res.digest
        );

        Ok(())
    }
}
