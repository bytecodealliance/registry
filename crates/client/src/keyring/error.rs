use std::fmt::Display;

/// Error returned when a keyring operation fails.
#[derive(Debug)]
pub struct KeyringError(KeyringErrorImpl);

#[derive(Debug)]
enum KeyringErrorImpl {
    UnknownBackend {
        backend: String,
    },
    BackendInitFailure {
        backend: &'static str,
        cause: KeyringErrorCause,
    },
    NoDefaultSigningKey {
        backend: &'static str,
    },
    AccessError {
        backend: &'static str,
        entry: KeyringEntry,
        action: KeyringAction,
        cause: KeyringErrorCause,
    },
}

#[derive(Debug)]
pub(super) enum KeyringErrorCause {
    Backend(keyring::Error),
    Other(anyhow::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum KeyringEntry {
    AuthToken { registry: String },
    SigningKey { registry: Option<String> },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum KeyringAction {
    Open,
    Get,
    Set,
    Delete,
}

impl From<keyring::Error> for KeyringErrorCause {
    fn from(error: keyring::Error) -> Self {
        KeyringErrorCause::Backend(error)
    }
}

impl From<anyhow::Error> for KeyringErrorCause {
    fn from(error: anyhow::Error) -> Self {
        KeyringErrorCause::Other(error)
    }
}

impl KeyringError {
    pub(super) fn unknown_backend(backend: String) -> Self {
        KeyringError(KeyringErrorImpl::UnknownBackend { backend })
    }

    pub(super) fn backend_init_failure(
        backend: &'static str,
        cause: impl Into<KeyringErrorCause>,
    ) -> Self {
        KeyringError(KeyringErrorImpl::BackendInitFailure {
            backend,
            cause: cause.into(),
        })
    }

    pub(super) fn no_default_signing_key(backend: &'static str) -> Self {
        KeyringError(KeyringErrorImpl::NoDefaultSigningKey { backend })
    }

    pub(super) fn auth_token_access_error(
        backend: &'static str,
        registry: &(impl Display + ?Sized),
        action: KeyringAction,
        cause: impl Into<KeyringErrorCause>,
    ) -> Self {
        KeyringError(KeyringErrorImpl::AccessError {
            backend,
            entry: KeyringEntry::AuthToken {
                registry: registry.to_string(),
            },
            action,
            cause: cause.into(),
        })
    }

    pub(super) fn signing_key_access_error(
        backend: &'static str,
        registry: Option<&(impl Display + ?Sized)>,
        action: KeyringAction,
        cause: impl Into<KeyringErrorCause>,
    ) -> Self {
        KeyringError(KeyringErrorImpl::AccessError {
            backend,
            entry: KeyringEntry::SigningKey {
                registry: registry.map(|s| s.to_string()),
            },
            action,
            cause: cause.into(),
        })
    }
}

impl std::fmt::Display for KeyringErrorCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyringErrorCause::Backend(e) => e.fmt(f),
            KeyringErrorCause::Other(e) => e.fmt(f),
        }
    }
}

impl std::fmt::Display for KeyringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "keyring error: ")?;
        match &self.0 {
            KeyringErrorImpl::UnknownBackend { backend } => {
                write!(f, "unknown backend '{backend}'. Run `warg config --keyring_backend <backend>` to configure a keyring backend supported on this platform.")
            }
            KeyringErrorImpl::BackendInitFailure { backend, .. } => {
                write!(f, "failed to initialize backend '{backend}'.")
            }
            KeyringErrorImpl::NoDefaultSigningKey { backend } => {
                let _ = backend;
                write!(f, "no default signing key is set. Please create one by running `warg key set <alg:base64>` or `warg key new`")
            }
            KeyringErrorImpl::AccessError {
                backend,
                entry,
                action,
                cause,
            } => {
                match *action {
                    KeyringAction::Open => write!(f, "failed to open ")?,
                    KeyringAction::Get => write!(f, "failed to read ")?,
                    KeyringAction::Set => write!(f, "failed to set ")?,
                    KeyringAction::Delete => write!(f, "failed to delete ")?,
                };
                match entry {
                    KeyringEntry::AuthToken { registry } => {
                        write!(f, "auth token for registry <{registry}>.")?
                    }
                    KeyringEntry::SigningKey {
                        registry: Some(registry),
                    } => write!(f, "signing key for registry <{registry}>.")?,
                    KeyringEntry::SigningKey { registry: None } => {
                        write!(f, "default signing key.")?
                    }
                };

                if *backend == "secret-service"
                    && matches!(
                        cause,
                        KeyringErrorCause::Backend(keyring::Error::PlatformFailure(_))
                    )
                {
                    write!(
                        f,
                        concat!(" Since you are using the 'secret-service' backend, ",
                        "the likely cause of this error is that no secret service ",
                        "implementation, such as GNOME Keyring or KWallet, is installed, ",
                        "or one is installed but not correctly configured. Consult your OS ",
                        "distribution's documentation for instructions on setting it up, or run ",
                        "`warg config --keyring_backend <backend>` to use a different backend.")
                    )?;
                }

                // The above will be followed by further information returned
                // from `self.source()`.
                Ok(())
            }
        }
    }
}

impl std::error::Error for KeyringError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            KeyringErrorImpl::AccessError { cause, .. }
            | KeyringErrorImpl::BackendInitFailure { cause, .. } => match cause {
                KeyringErrorCause::Backend(e) => Some(e),
                KeyringErrorCause::Other(e) => Some(e.as_ref()),
            },
            _ => None,
        }
    }
}
