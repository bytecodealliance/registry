use super::CommonOptions;
use anyhow::Result;
use clap::Args;
use warg_client::ClientError;
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
    pub version: Option<String>,
}

impl DownloadCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        println!("downloading package `{name}`...", name = self.name);

        // if user specifies exact verion, then set the `VersionReq` to exact match
        let version = match &self.version {
            Some(version) => VersionReq::parse(&format!("={}", version))?,
            None => VersionReq::STAR,
        };

        let res = client
            .download(&self.name, &version)
            .await?
            .ok_or_else(|| ClientError::PackageVersionRequirementDoesNotExist {
                name: self.name.clone(),
                version,
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
