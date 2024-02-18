use anyhow::{Context, Result};
use clap::Args;
use warg_client::RegistryUrl;

use crate::keyring::set_auth_token;

/// Manage signing keys for interacting with a registry.
#[derive(Args)]
pub struct LoginCommand {
    /// The subcommand to execute.
    // #[clap(subcommand)]
    // pub command: LogInSubcommand,
    #[clap(flatten)]
    keyring_entry: KeyringEntryArgs,
    // token: String,
}

#[derive(Args)]
struct KeyringEntryArgs {
    /// The name to use for the signing key.
    #[clap(long, short, value_name = "TOKEN_NAME", default_value = "default")]
    pub name: String,
    /// The URL of the registry to store an auth token for.
    #[clap(value_name = "URL")]
    pub url: RegistryUrl,
}

impl std::fmt::Display for KeyringEntryArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "`{name}` for registry `{url}`",
            name = self.name,
            url = self.url
        )
    }
}

impl KeyringEntryArgs {
    fn set_entry(&self, token: &str) -> Result<()> {
        set_auth_token(&self.url, &self.name, token)
    }
}

impl LoginCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let token = rpassword::prompt_password("Enter auth token:\n")
            .context("failed to read auth token")?;
        self.keyring_entry.set_entry(&token)?;
        println!(
            "auth token {keyring} was set successfully",
            keyring = self.keyring_entry
        );
        Ok(())
    }
}
