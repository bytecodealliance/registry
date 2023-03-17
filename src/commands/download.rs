use super::CommonOptions;
use anyhow::{anyhow, Result};
use clap::Args;
use warg_protocol::VersionReq;

/// Download a warg registry package.
#[derive(Args)]
#[clap(disable_version_flag = true)]
pub struct DownloadCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The name of the package to download.
    #[clap(value_name = "PACKAGE")]
    pub package: String,
    #[clap(long, short, value_name = "VERSION")]
    /// The version requirement of the package to download; defaults to `*`.
    pub version: Option<VersionReq>,
}

impl DownloadCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        println!("downloading package `{package}`...", package = self.package);
        let mut client = self.common.create_client().await?;

        let res = client
            .download(
                &self.package,
                self.version.as_ref().unwrap_or(&VersionReq::STAR),
            )
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "a version of package `{package}` that satisfies `{version}` was not found",
                    package = self.package,
                    version = self.version.as_ref().unwrap_or(&VersionReq::STAR)
                )
            })?;

        println!(
            "downloaded version {version} of package `{package}` ({digest})",
            package = self.package,
            version = res.version,
            digest = res.digest
        );

        Ok(())
    }
}
