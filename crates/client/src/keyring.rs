//! Utilities for interacting with keyring and performing signing operations.

use crate::config::Config;
use crate::RegistryUrl;
use anyhow::{anyhow, bail, Context};
use indexmap::IndexSet;
use secrecy::Secret;
use warg_crypto::signing::PrivateKey;

/// Interface to a pluggable keyring backend
#[derive(Debug)]
pub struct Keyring {
    imp: Box<keyring::CredentialBuilder>,
}

/// The type of keyring errors.
///
/// Currently just a synonym for [`anyhow::Error`], but will change to
/// something capable of more user-friendly diagnostics.
pub type KeyringError = anyhow::Error;

/// Result type for keyring errors.
pub type Result<T, E = KeyringError> = std::result::Result<T, E>;

impl Keyring {
    #[cfg(target_os = "linux")]
    /// List of supported credential store backends
    pub const SUPPORTED_BACKENDS: &'static [&'static str] =
        &["secret-service", "linux-keyutils", "mock"];
    #[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
    /// List of supported credential store backends
    pub const SUPPORTED_BACKENDS: &'static [&'static str] = &["secret-service", "mock"];
    #[cfg(target_os = "windows")]
    /// List of supported credential store backends
    pub const SUPPORTED_BACKENDS: &'static [&'static str] = &["windows", "mock"];
    #[cfg(target_os = "macos")]
    /// List of supported credential store backends
    pub const SUPPORTED_BACKENDS: &'static [&'static str] = &["macos", "mock"];
    #[cfg(target_os = "ios")]
    /// List of supported credential store backends
    pub const SUPPORTED_BACKENDS: &'static [&'static str] = &["ios", "mock"];
    #[cfg(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
    )))]
    /// List of supported credential store backends
    pub const SUPPORTED_BACKENDS: &'static [&'static str] = &["mock"];

    /// The default backend when no configuration option is set
    pub const DEFAULT_BACKEND: &'static str = Self::SUPPORTED_BACKENDS[0];

    /// Returns a human-readable description of a keyring backend.
    pub fn describe_backend(backend: &str) -> &'static str {
        match backend {
            "secret-service" => "Freedesktop.org secret service (GNOME Keyring or KWallet)",
            "linux-keyutils" => "Linux kernel memory-based keystore (lacks persistence, not suitable for desktop use)",
            "windows" => "Windows Credential Manager",
            "macos" => "MacOS Keychain",
            "ios" => "Apple iOS Keychain",
            "mock" => "Mock credential store with no persistence (for testing only)",
            _ => "(no description available)"
        }
    }

    fn load_backend(backend: &str) -> Option<Box<keyring::CredentialBuilder>> {
        if !Self::SUPPORTED_BACKENDS.contains(&backend) {
            return None;
        }

        #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
        if backend == "secret-service" {
            return Some(keyring::secret_service::default_credential_builder());
        }

        #[cfg(target_os = "linux")]
        if backend == "linux-keyutils" {
            return Some(keyring::keyutils::default_credential_builder());
        }

        #[cfg(target_os = "macos")]
        if backend == "macos" {
            return Some(keyring::macos::default_credential_builder());
        }

        #[cfg(target_os = "ios")]
        if backend == "ios" {
            return Some(keyring::ios::default_credential_builder());
        }

        #[cfg(target_os = "windows")]
        if backend == "windows" {
            return Some(keyring::windows::default_credential_builder());
        }

        if backend == "mock" {
            return Some(keyring::mock::default_credential_builder());
        }

        unreachable!("missing logic for backend {backend}")
    }

    /// Instantiate a new keyring.
    ///
    /// The argument should be an element of [Self::SUPPORTED_BACKENDS].
    pub fn new(backend: &str) -> Result<Self> {
        Self::load_backend(backend)
            .ok_or_else(|| anyhow!("failed to initialize keyring: unsupported backend {backend}"))
            .map(|imp| Self { imp })
    }

    /// Instantiate a new keyring using the backend specified in a configuration file.
    pub fn from_config(config: &Config) -> Result<Self> {
        if let Some(ref backend) = config.keyring_backend {
            Self::new(backend.as_str())
        } else {
            Self::new(Self::SUPPORTED_BACKENDS[0])
        }
    }

    /// Gets the auth token entry for the given registry and key name.
    pub fn get_auth_token_entry(&self, registry_url: &RegistryUrl) -> Result<keyring::Entry> {
        let label = format!("warg-auth-token:{}", registry_url.safe_label());
        let cred = self
            .imp
            .build(None, &label, &registry_url.safe_label())
            .context("failed to get keyring entry")?;
        Ok(keyring::Entry::new_with_credential(cred))
    }

    /// Gets the auth token
    pub fn get_auth_token(&self, registry_url: &RegistryUrl) -> Result<Option<Secret<String>>> {
        let entry = self.get_auth_token_entry(registry_url)?;
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
    pub fn delete_auth_token(&self, registry_url: &RegistryUrl) -> Result<()> {
        let entry = self.get_auth_token_entry(registry_url)?;
        match entry.delete_password() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => {
                bail!("no auth token found for registry `{registry_url}`");
            }
            Err(keyring::Error::Ambiguous(_)) => {
                bail!("more than one auth token found for registry `{registry_url}`");
            }
            Err(e) => {
                bail!("failed to delete auth token for registry `{registry_url}`: {e}");
            }
        }
    }

    /// Sets the auth token
    pub fn set_auth_token(&self, registry_url: &RegistryUrl, token: &str) -> Result<()> {
        let entry = self.get_auth_token_entry(registry_url)?;
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
        &self,
        registry_url: Option<&str>,
        keys: &IndexSet<String>,
        home_url: Option<&str>,
    ) -> Result<keyring::Entry> {
        if let Some(registry_url) = registry_url {
            let user = if keys.contains(registry_url) {
                registry_url
            } else {
                "default"
            };
            let cred = self
                .imp
                .build(None, "warg-signing-key", user)
                .context("failed to get keyring entry")?;
            Ok(keyring::Entry::new_with_credential(cred))
        } else {
            if let Some(url) = home_url {
                if keys.contains(url) {
                    let cred = self
                        .imp
                        .build(
                            None,
                            "warg-signing-key",
                            &RegistryUrl::new(url)?.safe_label(),
                        )
                        .context("failed to get keyring entry")?;
                    return Ok(keyring::Entry::new_with_credential(cred));
                }
            }

            if keys.contains("default") {
                let cred = self
                    .imp
                    .build(None, "warg-signing-key", "default")
                    .context("failed to get keyring entry")?;
                return Ok(keyring::Entry::new_with_credential(cred));
            }

            bail!("error: Please set a default signing key by typing `warg key set <alg:base64>` or `warg key new`");
        }
    }

    /// Gets the signing key for the given registry registry_label and key name.
    pub fn get_signing_key(
        &self,
        // If being called by a cli key command, this will always be a cli flag
        // If being called by a client publish command, this could also be supplied by namespace map config
        registry_url: Option<&str>,
        keys: &IndexSet<String>,
        home_url: Option<&str>,
    ) -> Result<PrivateKey> {
        let entry = self.get_signing_key_entry(registry_url, keys, home_url)?;

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
        &self,
        registry_url: Option<&str>,
        key: &PrivateKey,
        keys: &mut IndexSet<String>,
        home_url: Option<&str>,
    ) -> Result<()> {
        let entry = self.get_signing_key_entry(registry_url, keys, home_url)?;
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
        &self,
        registry_url: Option<&str>,
        keys: &IndexSet<String>,
        home_url: Option<&str>,
    ) -> Result<()> {
        let entry = self.get_signing_key_entry(registry_url, keys, home_url)?;

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
}
