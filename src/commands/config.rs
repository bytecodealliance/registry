use super::CommonOptions;
use anyhow::{bail, Context, Result};
use clap::Args;
use std::path::PathBuf;
use warg_client::{keyring::Keyring, Config, RegistryUrl};

/// Creates a new warg configuration file.
#[derive(Args)]
pub struct ConfigCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// The path to the registries directory to use.
    #[clap(long, value_name = "REGISTRIES")]
    pub registries_dir: Option<PathBuf>,

    /// The path to the content directory to use.
    #[clap(long, value_name = "CONTENT")]
    pub content_dir: Option<PathBuf>,

    /// Ignore federation hints.
    #[clap(long)]
    pub ignore_federation_hints: bool,

    /// Auto accept federation hints.
    #[clap(long)]
    pub auto_accept_federation_hints: bool,

    /// Overwrite the existing configuration file.
    #[clap(long)]
    pub overwrite: bool,

    /// The path to the configuration file to create.
    ///
    /// If not specified, the default of `$CONFIG_DIR/warg/config.json` is used.
    #[clap(value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// The path to the namespace map
    #[clap(long, value_name = "NAMESPACE_PATH")]
    pub namespace_path: Option<PathBuf>,

    /// The backend to use for keyring access
    #[clap(long, value_name = "KEYRING_BACKEND", value_parser = keyring_backend_parser, long_help = keyring_backend_help())]
    pub keyring_backend: Option<String>,
}

impl ConfigCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let path = self
            .path
            .map(Ok)
            .unwrap_or_else(Config::default_config_path)?;

        if !self.overwrite && path.is_file() {
            bail!(
                "configuration file `{path}` already exists; use `--overwrite` to overwrite it",
                path = path.display()
            );
        }

        let home_url = &self
            .common
            .registry
            .clone()
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
            home_url: home_url.clone(),
            registries_dir: self.registries_dir.map(|p| cwd.join(p)),
            content_dir: self.content_dir.map(|p| cwd.join(p)),
            namespace_map_path: self.namespace_path.map(|p| cwd.join(p)),
            keys: self.common.read_config()?.keys,
            keyring_auth: false,
            ignore_federation_hints: self.ignore_federation_hints,
            auto_accept_federation_hints: self.auto_accept_federation_hints,
            disable_interactive: false,
            keyring_backend: self.keyring_backend.clone(),
        };

        config.write_to_file(&path)?;

        // reset when changing home registry
        let client = self.common.create_client(&config)?;
        client.reset_namespaces().await?;
        client.reset_registry().await?;

        println!(
            "created warg configuration file `{path}`",
            path = path.display(),
        );

        Ok(())
    }
}

fn keyring_backend_parser(s: &str) -> Result<String, String> {
    if Keyring::SUPPORTED_BACKENDS.contains(&s) {
        Ok(s.to_string())
    } else {
        Err(format!("`{s}` is not a supported keyring backend."))
    }
}

fn keyring_backend_help() -> clap::builder::StyledStr {
    use std::fmt::Write as _;

    let mut help = String::new();

    writeln!(
        &mut help,
        "The backend to use for keyring access. The following options are supported:\n"
    )
    .unwrap();
    for backend in Keyring::SUPPORTED_BACKENDS {
        writeln!(
            &mut help,
            "{:16} {}",
            backend,
            Keyring::describe_backend(backend)
        )
        .unwrap();
    }
    writeln!(
        &mut help,
        "\nThe default is `{}`.",
        Keyring::DEFAULT_BACKEND
    )
    .unwrap();

    help.into()
}
