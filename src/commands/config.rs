use anyhow::{bail, Context, Result};
use clap::Args;
use serde_json;
use std::{collections::HashMap, fs, path::PathBuf, str::FromStr};
use warg_client::{Config, RegistryUrl};

#[derive(Clone)]
struct Namespace {
    namespace: String,
    domain: String,
}

impl FromStr for Namespace {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        let mut split = s.split("=");
        let namespace = split.next();
        let domain = split.next();
        if let (Some(namespace), Some(domain)) = (namespace, domain) {
            Ok(Namespace {
                namespace: namespace.to_owned(),
                domain: domain.to_owned(),
            })
        } else {
            bail!("expected namesape argument to be of form <namespace>=<domain>");
        }
    }
}
/// Creates a new warg configuration file.
#[derive(Args)]
pub struct ConfigCommand {
    /// The default registry URL to use.
    #[clap(long, value_name = "URL")]
    pub registry: Option<String>,

    /// The path to the registries directory to use.
    #[clap(long, value_name = "STORAGE")]
    pub storage_dir: Option<PathBuf>,

    /// The path to the registries directory to use.
    #[clap(long, value_name = "REGISTRIES")]
    pub registries_dir: Option<PathBuf>,

    /// The path to the content directory to use.
    #[clap(long, value_name = "CONTENT")]
    pub content_dir: Option<PathBuf>,

    /// The namespace and domain to map
    #[clap(long, long, value_name = "NAMESPACE")]
    namespace: Option<Namespace>,

    /// The path to the content directory to use.
    #[clap(long, value_name = "NAMESPACE_PATH")]
    pub namespace_path: Option<PathBuf>,

    /// Overwrite the existing configuration file.
    #[clap(long)]
    pub overwrite: bool,

    /// The path to the configuration file to create.
    ///
    /// If not specified, the default of `$CONFIG_DIR/warg/config.json` is used.
    #[clap(value_name = "PATH")]
    pub path: Option<PathBuf>,
}

impl ConfigCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let path = self
            .path
            .map(Ok)
            .unwrap_or_else(Config::default_config_path)?;

        if !self.overwrite && path.is_file() && self.namespace.is_none() {
            bail!(
                "configuration file `{path}` already exists; use `--overwrite` to overwrite it",
                path = path.display()
            );
        }

        let default_url = self
            .registry
            .map(RegistryUrl::new)
            .transpose()?
            .map(|u| u.to_string());

        // The paths specified on the command line are relative to the current
        // directory.
        //
        // `write_to_file` will handle normalizing the paths to be relative to
        // the configuration file's directory.
        let cwd = std::env::current_dir().context("failed to determine current directory")?;
        let config = Config {
            default_url,
            storage_dir: Some(cwd.clone()),
            registries_dir: self.registries_dir.map(|p| cwd.join(p)),
            content_dir: self.content_dir.map(|p| cwd.join(p)),
            namespace_map_path: self.namespace_path.map(|p| cwd.join(p)),
        };

        let mut namespace_config: HashMap<String, String> =
            serde_json::from_slice(&fs::read(config.namespace_map_path()?)?)?;
        if let Some(nm) = self.namespace {
            namespace_config.insert(nm.namespace, nm.domain);
            fs::write(
                config.namespace_map_path()?,
                serde_json::to_string(&namespace_config)?,
            )?;
        }

        config.write_to_file(&path)?;

        println!(
            "created warg configuration file `{path}`",
            path = path.display(),
        );

        Ok(())
    }
}
