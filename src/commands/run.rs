use super::CommonOptions;
use crate::demo;
use anyhow::{anyhow, Result};
use clap::Args;
use warg_client::storage::ContentStorage;
use warg_protocol::{registry::PackageName, VersionReq};

/// Run a package.
#[derive(Args)]
#[clap(disable_version_flag = true)]
pub struct RunCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    /// The version requirement of the package to download; defaults to `*`.
    #[clap(long, short, value_name = "VERSION")]
    pub version: Option<VersionReq>,
    /// The package name to run.
    #[clap(value_name = "PACKAGE")]
    pub name: PackageName,
    /// The arguments to the package.
    pub args: Vec<String>,
}

impl RunCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

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
            "running version {version} of package `{name}` ({digest})",
            name = self.name,
            version = res.version,
            digest = res.digest
        );

        let path = client
            .content()
            .content_location(&res.digest)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "content digest `{digest}` is not present in the local storage",
                    digest = res.digest
                )
            })?;

        demo::run_wasm(path, &self.args)
    }
}
