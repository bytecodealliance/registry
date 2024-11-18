//! Utilities for interacting with keyring and performing signing operations.

use crate::config::Config;
use crate::RegistryUrl;
use indexmap::IndexSet;
use secrecy::Secret;
use warg_crypto::signing::PrivateKey;

mod error;
use error::KeyringAction;
pub use error::KeyringError;

pub mod flatfile;

/// Interface to a pluggable keyring backend
#[derive(Debug)]
pub struct Keyring {
    imp: Box<keyring::CredentialBuilder>,
    name: &'static str,
}

/// Result type for keyring errors.
pub type Result<T, E = KeyringError> = std::result::Result<T, E>;

impl Keyring {
    #[cfg(target_os = "linux")]
    /// List of supported credential store backends
    pub const SUPPORTED_BACKENDS: &'static [&'static str] =
        &["secret-service", "flat-file", "linux-keyutils", "mock"];
    #[cfg(any(target_os = "freebsd", target_os = "openbsd"))]
    /// List of supported credential store backends
    pub const SUPPORTED_BACKENDS: &'static [&'static str] =
        &["secret-service", "flat-file", "mock"];
    #[cfg(target_os = "windows")]
    /// List of supported credential store backends
    pub const SUPPORTED_BACKENDS: &'static [&'static str] = &["windows", "flat-file", "mock"];
    #[cfg(target_os = "macos")]
    /// List of supported credential store backends
    pub const SUPPORTED_BACKENDS: &'static [&'static str] = &["macos", "flat-file", "mock"];
    #[cfg(target_os = "ios")]
    /// List of supported credential store backends
    pub const SUPPORTED_BACKENDS: &'static [&'static str] = &["ios", "flat-file", "mock"];
    #[cfg(not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "macos",
        target_os = "ios",
        target_os = "windows",
    )))]
    /// List of supported credential store backends
    pub const SUPPORTED_BACKENDS: &'static [&'static str] = &["flat-file", "mock"];

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
            "flat-file" => "Unencrypted flat files in your warg config directory",
            "mock" => "Mock credential store with no persistence (for testing only)",
            _ => "(no description available)"
        }
    }

    fn load_backend(backend: &str) -> Result<Box<keyring::CredentialBuilder>> {
        if !Self::SUPPORTED_BACKENDS.contains(&backend) {
            return Err(KeyringError::unknown_backend(backend.to_owned()));
        }

        #[cfg(any(target_os = "linux", target_os = "freebsd", target_os = "openbsd"))]
        if backend == "secret-service" {
            return Ok(keyring::secret_service::default_credential_builder());
        }

        #[cfg(target_os = "linux")]
        if backend == "linux-keyutils" {
            return Ok(keyring::keyutils::default_credential_builder());
        }

        #[cfg(target_os = "macos")]
        if backend == "macos" {
            return Ok(keyring::macos::default_credential_builder());
        }

        #[cfg(target_os = "ios")]
        if backend == "ios" {
            return Ok(keyring::ios::default_credential_builder());
        }

        #[cfg(target_os = "windows")]
        if backend == "windows" {
            return Ok(keyring::windows::default_credential_builder());
        }

        if backend == "flat-file" {
            return Ok(Box::new(
                flatfile::FlatfileCredentialBuilder::new()
                    .map_err(|e| KeyringError::backend_init_failure("flat-file", e))?,
            ));
        }

        if backend == "mock" {
            return Ok(keyring::mock::default_credential_builder());
        }

        unreachable!("missing logic for backend {backend}")
    }

    /// Instantiate a new keyring.
    ///
    /// The argument should be an element of [Self::SUPPORTED_BACKENDS].
    pub fn new(backend: &str) -> Result<Self> {
        Self::load_backend(backend).map(|imp| Self {
            imp,
            // Get an equivalent &'static str from our &str
            name: Self::SUPPORTED_BACKENDS
                .iter()
                .find(|s| **s == backend)
                .expect("successfully-loaded backend should be found in SUPPORTED_BACKENDS"),
        })
    }

    /// Instantiate a new keyring using the backend specified in a configuration file.
    pub fn from_config(config: &Config) -> Result<Self> {
        if let Some(ref backend) = config.keyring_backend {
            Self::new(backend.as_str())
        } else {
            Self::new(Self::DEFAULT_BACKEND)
        }
    }

    /// Gets the auth token entry for the given registry and key name.
    pub fn get_auth_token_entry(&self, registry_url: &RegistryUrl) -> Result<keyring::Entry> {
        let label = format!("warg-auth-token:{}", registry_url.safe_label());
        let cred = self
            .imp
            .build(None, &label, &registry_url.safe_label())
            .map_err(|e| {
                KeyringError::auth_token_access_error(
                    self.name,
                    registry_url,
                    KeyringAction::Open,
                    e,
                )
            })?;
        Ok(keyring::Entry::new_with_credential(cred))
    }

    /// Gets the auth token
    pub fn get_auth_token(&self, registry_url: &RegistryUrl) -> Result<Option<Secret<String>>> {
        let entry = self.get_auth_token_entry(registry_url)?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(Secret::from(secret))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(KeyringError::auth_token_access_error(
                self.name,
                registry_url,
                KeyringAction::Get,
                e,
            )),
        }
    }

    /// Deletes the auth token
    pub fn delete_auth_token(&self, registry_url: &RegistryUrl) -> Result<()> {
        let entry = self.get_auth_token_entry(registry_url)?;
        entry.delete_credential().map_err(|e| {
            KeyringError::auth_token_access_error(self.name, registry_url, KeyringAction::Delete, e)
        })
    }

    /// Sets the auth token
    pub fn set_auth_token(&self, registry_url: &RegistryUrl, token: &str) -> Result<()> {
        let entry = self.get_auth_token_entry(registry_url)?;
        entry.set_password(token).map_err(|e| {
            KeyringError::auth_token_access_error(self.name, registry_url, KeyringAction::Set, e)
        })
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
                .map_err(|e| {
                    KeyringError::signing_key_access_error(
                        self.name,
                        Some(registry_url),
                        KeyringAction::Open,
                        e,
                    )
                })?;
            Ok(keyring::Entry::new_with_credential(cred))
        } else {
            if let Some(url) = home_url {
                if keys.contains(url) {
                    let cred = self
                        .imp
                        .build(
                            None,
                            "warg-signing-key",
                            &RegistryUrl::new(url)
                                .map_err(|e| {
                                    KeyringError::signing_key_access_error(
                                        self.name,
                                        Some(url),
                                        KeyringAction::Open,
                                        e,
                                    )
                                })?
                                .safe_label(),
                        )
                        .map_err(|e| {
                            KeyringError::signing_key_access_error(
                                self.name,
                                Some(url),
                                KeyringAction::Open,
                                e,
                            )
                        })?;
                    return Ok(keyring::Entry::new_with_credential(cred));
                }
            }

            if keys.contains("default") {
                let cred = self
                    .imp
                    .build(None, "warg-signing-key", "default")
                    .map_err(|e| {
                        KeyringError::signing_key_access_error(
                            self.name,
                            None::<&str>,
                            KeyringAction::Open,
                            e,
                        )
                    })?;
                return Ok(keyring::Entry::new_with_credential(cred));
            }

            Err(KeyringError::no_default_signing_key(self.name))
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
            Ok(secret) => PrivateKey::decode(secret).map_err(|e| {
                KeyringError::signing_key_access_error(
                    self.name,
                    registry_url,
                    KeyringAction::Get,
                    anyhow::Error::from(e),
                )
            }),
            Err(e) => Err(KeyringError::signing_key_access_error(
                self.name,
                registry_url,
                KeyringAction::Get,
                e,
            )),
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
        entry.set_password(&key.encode()).map_err(|e| {
            KeyringError::signing_key_access_error(self.name, registry_url, KeyringAction::Set, e)
        })
    }

    /// Deletes the signing key for the given registry host and key name.
    pub fn delete_signing_key(
        &self,
        registry_url: Option<&str>,
        keys: &IndexSet<String>,
        home_url: Option<&str>,
    ) -> Result<()> {
        let entry = self.get_signing_key_entry(registry_url, keys, home_url)?;
        entry.delete_credential().map_err(|e| {
            KeyringError::signing_key_access_error(
                self.name,
                registry_url,
                KeyringAction::Delete,
                e,
            )
        })
    }
}
