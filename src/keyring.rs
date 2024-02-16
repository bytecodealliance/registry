//! Utilities for interacting with keyring and performing signing operations.

use anyhow::{bail, Context, Result};
use keyring::Entry;
use warg_client::RegistryUrl;
use warg_crypto::signing::PrivateKey;

/// Gets the auth token entry for the given registry and key name.
pub fn get_auth_token_entry(registry_url: &RegistryUrl, token_name: &str) -> Result<Entry> {
    let label = format!("warg-auth-token:{}", registry_url.safe_label());
    Entry::new(&label, token_name).context("failed to get keyring entry")
}

/// Gets the auth token
pub fn get_auth_token(registry_url: &RegistryUrl, token_name: &str) -> Result<String> {
    let entry = get_signing_key_entry(registry_url, token_name)?;
    match entry.get_password() {
        Ok(secret) => Ok(secret),
        Err(keyring::Error::NoEntry) => {
            bail!("no signing key found with name `{token_name}` of registry `{registry_url}`");
        }
        Err(keyring::Error::Ambiguous(_)) => {
            bail!("more than one signing key found with name `{token_name}` of registry `{registry_url}`");
        }
        Err(e) => {
            bail!("failed to get signing key with name `{token_name}` of registry `{registry_url}`: {e}");
        }
    }
}

/// Sets the auth token
pub fn set_auth_token(registry_url: &RegistryUrl, token_name: &str, token: &str) -> Result<()> {
    let entry = get_auth_token_entry(registry_url, token_name)?;
    match entry.set_password(token) {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => {
            bail!("no auth token found with name `{token_name}` of registry `{registry_url}`");
        }
        Err(keyring::Error::Ambiguous(_)) => {
            bail!("more than one auth token with name `{token_name}` of registry `{registry_url}`");
        }
        Err(e) => {
            bail!("failed to set auth token with name `{token_name}` of registry `{registry_url}`: {e}");
        }
    }
}

/// Gets the signing key entry for the given registry and key name.
pub fn get_signing_key_entry(registry_url: &RegistryUrl, key_name: &str) -> Result<Entry> {
    let label = format!("warg-signing-key:{}", registry_url.safe_label());
    Entry::new(&label, key_name).context("failed to get keyring entry")
}

/// Gets the signing key for the given registry registry_label and key name.
pub fn get_signing_key(registry_url: &RegistryUrl, key_name: &str) -> Result<PrivateKey> {
    let entry = get_signing_key_entry(registry_url, key_name)?;

    match entry.get_password() {
        Ok(secret) => PrivateKey::decode(secret).context("failed to parse signing key"),
        Err(keyring::Error::NoEntry) => {
            bail!("no signing key found with name `{key_name}` of registry `{registry_url}`");
        }
        Err(keyring::Error::Ambiguous(_)) => {
            bail!("more than one signing key found with name `{key_name}` of registry `{registry_url}`");
        }
        Err(e) => {
            bail!("failed to get signing key with name `{key_name}` of registry `{registry_url}`: {e}");
        }
    }
}

/// Sets the signing key for the given registry host and key name.
pub fn set_signing_key(registry_url: &RegistryUrl, key_name: &str, key: &PrivateKey) -> Result<()> {
    let entry = get_signing_key_entry(registry_url, key_name)?;
    match entry.set_password(&key.encode()) {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => {
            bail!("no signing key found with name `{key_name}` of registry `{registry_url}`");
        }
        Err(keyring::Error::Ambiguous(_)) => {
            bail!("more than one signing key found with name `{key_name}` of registry `{registry_url}`");
        }
        Err(e) => {
            bail!("failed to set signing key with name `{key_name}` of registry `{registry_url}`: {e}");
        }
    }
}

/// Deletes the signing key for the given registry host and key name.
pub fn delete_signing_key(registry_url: &RegistryUrl, key_name: &str) -> Result<()> {
    let entry = get_signing_key_entry(registry_url, key_name)?;
    match entry.delete_password() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => {
            bail!("no signing key found with name `{key_name}` of registry `{registry_url}`");
        }
        Err(keyring::Error::Ambiguous(_)) => {
            bail!("more than one signing key found with name `{key_name}` of registry `{registry_url}`");
        }
        Err(e) => {
            bail!("failed to set signing key with name `{key_name}` of registry `{registry_url}`: {e}");
        }
    }
}
