//! Flat-file keyring backend
//!
//! This backend stores credentials as unencrypted flat files in the user's
//! configuration directory. It is portable to all platforms, but the lack of
//! encryption can make it a less secure option than the platform-specific
//! encrypted backends such as `secret-service`.

use keyring::credential::{Credential, CredentialApi, CredentialBuilderApi};

use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use url::form_urlencoded::Serializer;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

/// Builder for flat-file credentials
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FlatfileCredentialBuilder(PathBuf);

/// A credential stored in a flat file
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FlatfileCredential(PathBuf);

impl FlatfileCredentialBuilder {
    /// Construct the credential builder, storing credentials in
    /// `$XDG_CONFIG_HOME/warg/keyring`.
    pub fn new() -> keyring::Result<Self> {
        let dir = dirs::config_dir()
            .ok_or(keyring::Error::NoEntry)?
            .join("warg")
            .join("keyring");
        Self::new_with_basepath(dir)
    }

    /// Construct the credential builder, storing all credentials in the
    /// given directory. The directory will be created if it is does not exist.
    pub fn new_with_basepath(basepath: PathBuf) -> keyring::Result<Self> {
        std::fs::create_dir_all(basepath.as_path())
            .map_err(|e| keyring::Error::PlatformFailure(Box::new(e)))?;

        #[cfg(unix)]
        std::fs::set_permissions(basepath.as_path(), std::fs::Permissions::from_mode(0o700))
            .map_err(|e| keyring::Error::PlatformFailure(Box::new(e)))?;

        Ok(Self(basepath))
    }
}

impl CredentialBuilderApi for FlatfileCredentialBuilder {
    fn build(
        &self,
        target: Option<&str>,
        service: &str,
        user: &str,
    ) -> keyring::Result<Box<Credential>> {
        let mut serializer = Serializer::new(String::new());
        if let Some(target) = target {
            serializer.append_pair("target", target);
        }
        serializer.append_pair("service", service);
        serializer.append_pair("user", user);

        let filename = serializer.finish();

        let path = self.0.join(filename);
        Ok(Box::new(FlatfileCredential(path)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl CredentialApi for FlatfileCredential {
    fn set_password(&self, password: &str) -> keyring::Result<()> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        options.mode(0o600);

        let mut f = options
            .open(self.0.as_path())
            .map_err(|e| keyring::Error::PlatformFailure(Box::new(e)))?;
        f.write_all(password.as_bytes())
            .map_err(|e| keyring::Error::PlatformFailure(Box::new(e)))?;
        Ok(())
    }

    fn get_password(&self) -> keyring::Result<String> {
        match File::open(self.0.as_path()) {
            Ok(mut f) => {
                let mut buf = Vec::new();
                f.read_to_end(&mut buf)
                    .map_err(|e| keyring::Error::PlatformFailure(Box::new(e)))?;
                String::from_utf8(buf).map_err(|e| keyring::Error::BadEncoding(e.into_bytes()))
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(keyring::Error::NoEntry)
                } else {
                    Err(keyring::Error::PlatformFailure(Box::new(e)))
                }
            }
        }
    }

    fn delete_password(&self) -> keyring::Result<()> {
        match std::fs::remove_file(self.0.as_path()) {
            Ok(()) => Ok(()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(keyring::Error::NoEntry)
                } else {
                    Err(keyring::Error::PlatformFailure(Box::new(e)))
                }
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn test_smoke() {
    let basepath = tempfile::tempdir().unwrap();
    let keyring =
        FlatfileCredentialBuilder::new_with_basepath(basepath.as_ref().to_owned()).unwrap();
    let cred = keyring.build(None, "service1", "user1").unwrap();
    assert!(matches!(
        cred.get_password().unwrap_err(),
        keyring::Error::NoEntry
    ));
    cred.set_password("correct horse battery staple").unwrap();
    assert_eq!(cred.get_password().unwrap(), "correct horse battery staple");

    let _dirattr = std::fs::metadata(basepath.as_ref()).unwrap();
    #[cfg(unix)]
    assert_eq!(_dirattr.permissions().mode() & 0o7777, 0o700);

    let filepath = basepath.as_ref().join("service=service1&user=user1");
    let _fileattr = std::fs::metadata(filepath.as_path()).unwrap();
    #[cfg(unix)]
    assert_eq!(_fileattr.permissions().mode() & 0o7777, 0o600);

    cred.delete_password().unwrap();
    assert!(matches!(
        cred.get_password().unwrap_err(),
        keyring::Error::NoEntry
    ));
}
