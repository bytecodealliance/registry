use super::CommonOptions;
use crate::demo;
use anyhow::{anyhow, Result};
use clap::Args;
use warg_protocol::VersionReq;

/// Run a package.
#[derive(Args)]
#[clap(disable_version_flag = true)]
pub struct RunCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,
    #[clap(long, short, value_name = "VERSION")]
    /// The version requirement of the package to download; defaults to `*`.
    pub version: Option<VersionReq>,
    #[clap(value_name = "NAME")]
    /// The name of the package to run.
    pub name: String,
    /// The arguments to the package.
    pub args: Vec<String>,
}

impl RunCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        println!("downloading package `{name}`...", name = self.name);

        let mut client = self.common.create_client().await?;

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
            "running version {version} of package `{package}` ({digest})",
            package = self.name,
            version = res.version,
            digest = res.digest
        );

        let path = client
            .storage()
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
