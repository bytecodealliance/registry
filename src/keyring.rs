//! Utilities for interacting with keyring and performing signing operations.

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use keyring::Entry;
use secrecy::{self, Secret};
use warg_client::{Config, CredList, RegistryUrl};
use warg_crypto::signing::PrivateKey;

/// Gets the auth token entry for the given registry and key name.
pub fn get_auth_token_entry(registry_url: &RegistryUrl, token_name: &str) -> Result<Entry> {
    let label = format!("warg-auth-token:{}", registry_url.safe_label());
    Entry::new(&label, token_name).context("failed to get keyring entry")
}

/// Gets the auth token
pub fn get_auth_token(registry_url: &RegistryUrl, token_name: &str) -> Result<Secret<String>> {
    let entry = get_auth_token_entry(registry_url, token_name)?;
    match entry.get_password() {
        Ok(secret) => Ok(Secret::from(secret)),
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
pub fn get_signing_key_entry(
    registry_url: RegistryUrl,
    key_name: &str,
    config: &Config,
) -> Result<Entry> {
    dbg!(&registry_url, key_name);
    if let Some(creds) = config.creds {
        if let Some(reg_keys) = creds.keys.get(&registry_url.safe_label()) {
            if let Some(key_name) = reg_keys.get(key_name) {
                Entry::new(
                    &format!("warg-signing-key:{}", registry_url.safe_label()),
                    key_name,
                )
                .context("failed to get keyring entry")
            } else {
                Entry::new(
                    &format!("warg-signing-key:{}", registry_url.safe_label()),
                    "default",
                )
                .context("failed to get keyring entry")
            }
        } else if let Some(default_keys) = creds.keys.get(&registry_url.safe_label()) {
            if let Some(key_name) = default_keys.get(key_name) {
                Entry::new(
                    &format!("warg-signing-key:{}", registry_url.safe_label()),
                    key_name,
                )
                .context("failed to get keyring entry")
            } else {
                Entry::new(
                    &format!("warg-signing-key:{}", registry_url.safe_label()),
                    "default",
                )
                .context("failed to get keyring entry")
            }
        } else {
            Entry::new("warg-signing-key:default", key_name).context("failed to get keyring entry")
        }
    } else {
        Entry::new("warg-signing-key:default", key_name).context("failed to get keyring entry")
    }
}

/// Gets the signing key for the given registry registry_label and key name.
pub fn get_signing_key(
    registry_url: RegistryUrl,
    key_name: &str,
    config: &Config,
) -> Result<PrivateKey> {
    let entry = get_signing_key_entry(registry_url, key_name, config)?;

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
pub fn set_signing_key(
    registry_url: &Option<RegistryUrl>,
    key_name: &str,
    key: &PrivateKey,
    config: &mut Config,
) -> Result<()> {
    match (registry_url, &mut config.creds) {
        (None, None) => {
            let mut registries = HashMap::new();
            let mut keys = HashMap::new();

            keys.insert("default".to_string(), key_name.to_string());
            registries.insert("default".to_string(), keys);
            config.creds = Some(CredList {
                keys: registries,
                tokens: vec![],
            });
            config.write_to_file(&Config::default_config_path()?)?;
        }
        (None, Some(creds)) => {
            if let Some(default_keys) = creds.clone().keys.get_mut("default") {
                default_keys.insert(key_name.to_string(), key_name.to_string());
                creds
                    .keys
                    .insert("default".to_string(), default_keys.clone());
            } else {
                let mut default_keys = HashMap::new();
                default_keys.insert(key_name.to_string(), key_name.to_string());
                creds
                    .keys
                    .insert("default".to_string(), default_keys.clone());
            };
            config.write_to_file(&Config::default_config_path()?)?;
        }
        (Some(reg), None) => {
            let mut registries = HashMap::new();
            let mut keys = HashMap::new();

            keys.insert(reg.safe_label().to_string(), key_name.to_string());
            registries.insert(reg.safe_label().to_string(), keys);
            config.creds = Some(CredList {
                keys: registries,
                tokens: vec![],
            });
            config.write_to_file(&Config::default_config_path()?)?;
        }
        (Some(reg), Some(creds)) => {
            if let Some(reg_keys) = creds.clone().keys.get_mut(&reg.safe_label()) {
                reg_keys.insert(key_name.to_string(), key_name.to_string());
                creds
                    .keys
                    .insert(reg.safe_label().to_string(), reg_keys.clone());
            } else {
                let mut reg_keys = HashMap::new();
                reg_keys.insert(key_name.to_string(), key_name.to_string());
                creds.keys.insert(reg.safe_label().to_string(), reg_keys);
            };
            config.write_to_file(&Config::default_config_path()?)?;
        }
    }
    let entry = get_signing_key_entry(registry_url, key_name, config)?;
    match entry.set_password(&key.encode()) {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => {
            if let Some(reg) = registry_url {
                bail!("no signing key found with name `{key_name}` of registry `{reg}`");
            } else {
                bail!("no signing key found with name `{key_name}`");
            }
        }
        Err(keyring::Error::Ambiguous(_)) => {
            if let Some(reg) = registry_url {
                bail!("more than one signing key found with name `{key_name}` of registry `{reg}`");
            } else {
                bail!("more than one signing key found with name `{key_name}`");
            }
        }
        Err(e) => {
            if let Some(reg) = registry_url {
                bail!("failed to get signing key with name `{key_name}` of registry `{reg}`: {e}");
            } else {
                bail!("failed to get signing key with name `{key_name}`: {e}");
            }
        }
    }
}

/// Deletes the signing key for the given registry host and key name.
pub fn delete_signing_key(
    registry_url: &Option<RegistryUrl>,
    key_name: &str,
    config: &Config,
) -> Result<()> {
    let entry = get_signing_key_entry(registry_url, key_name, config)?;
    match entry.delete_password() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => {
            if let Some(reg) = registry_url {
                bail!("no signing key found with name `{key_name}` of registry `{reg}`");
            } else {
                bail!("no signing key found with name `{key_name}`");
            }
        }
        Err(keyring::Error::Ambiguous(_)) => {
            if let Some(reg) = registry_url {
                bail!("more than one signing key found with name `{key_name}` of registry `{reg}`");
            } else {
                bail!("more than one signing key found with name `{key_name}`");
            }
        }
        Err(e) => {
            if let Some(reg) = registry_url {
                bail!("failed to get signing key with name `{key_name}` of registry `{reg}`: {e}");
            } else {
                bail!("failed to get signing key with name `{key_name}`: {e}");
            }
        }
    }
}
