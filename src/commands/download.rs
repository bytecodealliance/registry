use super::CommonOptions;
use anyhow::Result;
use clap::Args;
use dialoguer::{theme::ColorfulTheme, Confirm};
use std::path::PathBuf;
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
    /// The version requirement of the package to download; defaults to `*`.
    #[clap(long, short, value_name = "VERSION")]
    pub version: Option<String>,
    /// The output path for the file. If not specified, just downloads to local cache.
    #[clap(long, short = 'o')]
    pub output: Option<PathBuf>,
}

impl DownloadCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config).await?;

        println!("Downloading `{name}`...", name = self.name);

        // if user specifies exact verion, then set the `VersionReq` to exact match
        let version = match &self.version {
            Some(version) => VersionReq::parse(&format!("={}", version))?,
            None => VersionReq::STAR,
        };

        let download = client
            .download(&self.name, &version)
            .await?
            .ok_or_else(|| ClientError::PackageVersionRequirementDoesNotExist {
                name: self.name.clone(),
                version,
            })?;

        println!(
            "Downloaded version: {version}\nDigest: {digest}\n",
            version = download.version,
            digest = download.digest
        );

        // use the `output` path specified or ask the use if wants to save in the current working
        // directory
        let default_file_name = format!("{name}.wasm", name = self.name.name());
        if let Some(path) = self
            .output
            .map(|mut p| {
                if p.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("wasm"))
                {
                    p
                } else {
                    p.push(&default_file_name);
                    p
                }
            })
            .or_else(|| {
                if Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt(format!(
                        "Write `{default_file_name}` in current directory? y/N\n",
                    ))
                    .default(true)
                    .interact()
                    .unwrap()
                {
                    Some(PathBuf::from(default_file_name))
                } else {
                    None
                }
            })
        {
            std::fs::copy(download.path, &path)?;
            println!(
                "Wrote `{name}` to {path}",
                name = self.name,
                path = path.display(),
            );
        }

        Ok(())
    }
}
