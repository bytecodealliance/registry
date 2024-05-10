//! Utilities for interacting with keyring and performing signing operations.

use crate::RegistryUrl;
use anyhow::{bail, Context, Result};
use indexmap::IndexSet;
use secrecy::Secret;
use warg_crypto::signing::PrivateKey;

/// Gets the auth token entry for the given registry and key name.
pub fn get_auth_token_entry(registry_url: &RegistryUrl) -> Result<keyring::Entry> {
    let label = format!("warg-auth-token:{}", registry_url.safe_label());
    keyring::Entry::new(&label, &registry_url.safe_label()).context("failed to get keyring entry")
}

/// Gets the auth token
pub fn get_auth_token(registry_url: &RegistryUrl) -> Result<Option<Secret<String>>> {
    let entry = get_auth_token_entry(registry_url)?;
    match entry.get_password() {
        Ok(secret) => Ok(Some(Secret::from(secret))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(keyring::Error::Ambiguous(_)) => {
            bail!("more than one auth token for registry `{registry_url}`");
        }
        Err(e) => {
            bail!("failed to get auth token for registry `{registry_url}`: {e}");
        }
    }
}

/// Deletes the auth token
pub fn delete_auth_token(registry_url: &RegistryUrl) -> Result<()> {
    let entry = get_auth_token_entry(registry_url)?;
    match entry.delete_password() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => {
            bail!("no auth token found for registry `{registry_url}`");
        }
        Err(keyring::Error::Ambiguous(_)) => {
            bail!("more than one auth token found for registry `{registry_url}`");
        }
        Err(e) => {
            bail!("failed to delete auth torkn for registry `{registry_url}`: {e}");
        }
    }
}

/// Sets the auth token
pub fn set_auth_token(registry_url: &RegistryUrl, token: &str) -> Result<()> {
    let entry = get_auth_token_entry(registry_url)?;
    match entry.set_password(token) {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => {
            bail!("no auth token found for registry `{registry_url}`");
        }
        Err(keyring::Error::Ambiguous(_)) => {
            bail!("more than one auth token for registry `{registry_url}`");
        }
        Err(e) => {
            bail!("failed to set auth token for registry `{registry_url}`: {e}");
        }
    }
}

/// Gets the signing key entry for the given registry and key name.
pub fn get_signing_key_entry(
    registry_url: Option<&str>,
    keys: &IndexSet<String>,
    home_url: Option<&str>,
) -> Result<keyring::Entry> {
    if let Some(registry_url) = registry_url {
        if keys.contains(registry_url) {
            keyring::Entry::new("warg-signing-key", registry_url)
                .context("failed to get keyring entry")
        } else {
            keyring::Entry::new("warg-signing-key", "default")
                .context("failed to get keyring entry")
        }
    } else {
        if let Some(url) = home_url {
            if keys.contains(url) {
                return keyring::Entry::new(
                    "warg-signing-key",
                    &RegistryUrl::new(url)?.safe_label(),
                )
                .context("failed to get keyring entry");
            }
        }
        if keys.contains("default") {
            keyring::Entry::new("warg-signing-key", "default")
                .context("failed to get keyring entry")
        } else {
            bail!(
                        "error: Please set a default signing key by typing `warg key set <alg:base64>` or `warg key new`"
                    )
        }
    }
}

/// Gets the signing key for the given registry registry_label and key name.
pub fn get_signing_key(
    // If being called by a cli key command, this will always be a cli flag
    // If being called by a client publish command, this could also be supplied by namespace map config
    registry_url: Option<&str>,
    keys: &IndexSet<String>,
    home_url: Option<&str>,
) -> Result<PrivateKey> {
    let entry = get_signing_key_entry(registry_url, keys, home_url)?;

    match entry.get_password() {
        Ok(secret) => PrivateKey::decode(secret).context("failed to parse signing key"),
        Err(keyring::Error::NoEntry) => {
            if let Some(registry_url) = registry_url {
                bail!("no signing key found for registry `{registry_url}`");
            } else {
                bail!("no signing key found");
            }
        }
        Err(keyring::Error::Ambiguous(_)) => {
            if let Some(registry_url) = registry_url {
                bail!("more than one signing key found for registry `{registry_url}`");
            } else {
                bail!("more than one signing key found");
            }
        }
        Err(e) => {
            if let Some(registry_url) = registry_url {
                bail!("failed to get signing key for registry `{registry_url}`: {e}");
            } else {
                bail!("failed to get signing key`");
            }
        }
    }
}

/// Sets the signing key for the given registry host and key name.
pub fn set_signing_key(
    registry_url: Option<&str>,
    key: &PrivateKey,
    keys: &mut IndexSet<String>,
    home_url: Option<&str>,
) -> Result<()> {
    let entry = get_signing_key_entry(registry_url, keys, home_url)?;
    match entry.set_password(&key.encode()) {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => {
            if let Some(registry_url) = registry_url {
                bail!("no signing key found for registry `{registry_url}`");
            } else {
                bail!("no signing key found`");
            }
        }
        Err(keyring::Error::Ambiguous(_)) => {
            if let Some(registry_url) = registry_url {
                bail!("more than one signing key found for registry `{registry_url}`");
            } else {
                bail!("more than one signing key found");
            }
        }
        Err(e) => {
            if let Some(registry_url) = registry_url {
                bail!("failed to get signing key for registry `{registry_url}`: {e}");
            } else {
                bail!("failed to get signing: {e}");
            }
        }
    }
}

/// Deletes the signing key for the given registry host and key name.
pub fn delete_signing_key(
    registry_url: Option<&str>,
    keys: &IndexSet<String>,
    home_url: Option<&str>,
) -> Result<()> {
    let entry = get_signing_key_entry(registry_url, keys, home_url)?;

    match entry.delete_password() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => {
            if let Some(registry_url) = registry_url {
                bail!("no signing key found for registry `{registry_url}`");
            } else {
                bail!("no signing key found");
            }
        }
        Err(keyring::Error::Ambiguous(_)) => {
            if let Some(registry_url) = registry_url {
                bail!("more than one signing key found for registry `{registry_url}`");
            } else {
                bail!("more than one signing key found`");
            }
        }
        Err(e) => {
            if let Some(registry_url) = registry_url {
                bail!("failed to delete signing key for registry `{registry_url}`: {e}");
            } else {
                bail!("failed to delete signing key");
            }
        }
    }
}
