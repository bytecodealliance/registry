use super::{RecordPolicy, RecordPolicyError, RecordPolicyResult};
use anyhow::{bail, Result};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use warg_crypto::signing::KeyID;
use warg_protocol::{package::PackageRecord, registry::PackageId, ProtoEnvelope};
use wasmparser::names::KebabStr;

/// A policy that ensures a published record is authorized by
/// the key signing the record.
#[derive(Default, Deserialize)]
pub struct AuthorizedKeyPolicy {
    #[serde(skip)]
    superuser_keys: HashSet<KeyID>,
    #[serde(default)]
    namespace_keys: HashMap<String, HashSet<KeyID>>,
    #[serde(default)]
    package_keys: HashMap<PackageId, HashSet<KeyID>>,
}

impl AuthorizedKeyPolicy {
    /// Creates a new authorized key policy.
    ///
    /// By default, no keys are authorized.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets an authorized key for publishing to any namespace.
    pub fn with_superuser_key(mut self, key: KeyID) -> Self {
        self.superuser_keys.insert(key);
        self
    }

    /// Sets an authorized key for publishing to a particular namespace.
    pub fn with_namespace_key(mut self, namespace: impl Into<String>, key: KeyID) -> Result<Self> {
        let namespace = namespace.into();
        if KebabStr::new(&namespace).is_none() {
            bail!("namespace `{namespace}` is not a legal kebab-case identifier");
        }

        self.namespace_keys
            .entry(namespace)
            .or_default()
            .insert(key);
        Ok(self)
    }

    pub fn with_package_key(mut self, package_id: impl Into<String>, key: KeyID) -> Result<Self> {
        let package_id = PackageId::new(package_id)?;
        self.package_keys.entry(package_id).or_default().insert(key);
        Ok(self)
    }

    pub fn key_authorized_for_package(&self, key: &KeyID, package: &PackageId) -> bool {
        if self.superuser_keys.contains(key) {
            return true;
        }

        let namespace_keys = self.namespace_keys.get(package.namespace());
        if namespace_keys
            .map(|keys| keys.contains(key))
            .unwrap_or(false)
        {
            return true;
        }

        let package_keys = self.package_keys.get(package);
        if package_keys.map(|keys| keys.contains(key)).unwrap_or(false) {
            return true;
        }

        false
    }
}

impl RecordPolicy for AuthorizedKeyPolicy {
    fn check(
        &self,
        id: &PackageId,
        record: &ProtoEnvelope<PackageRecord>,
    ) -> RecordPolicyResult<()> {
        let key = record.key_id();
        if !self.key_authorized_for_package(key, id) {
            return Err(RecordPolicyError::Unauthorized(format!(
                "key id `{key}` is not authorized to publish to package `{id}`",
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_authorized_for_package() -> Result<()> {
        let super_key = KeyID::from("super-key".to_string());
        let namespace_key = KeyID::from("namespace-key".to_string());
        let package_key = KeyID::from("package-key".to_string());
        let other_key = KeyID::from("other-key".to_string());

        let policy = AuthorizedKeyPolicy::new()
            .with_superuser_key(super_key.clone())
            .with_namespace_key("my-namespace", namespace_key.clone())?
            .with_package_key("my-namespace:my-package", package_key.clone())?;

        let my_package: PackageId = "my-namespace:my-package".parse()?;
        let my_namespace_other_package: PackageId = "my-namespace:other-package".parse()?;
        let other_namespace: PackageId = "other-namespace:any-package".parse()?;

        assert!(policy.key_authorized_for_package(&super_key, &my_package));
        assert!(policy.key_authorized_for_package(&super_key, &my_namespace_other_package));
        assert!(policy.key_authorized_for_package(&super_key, &other_namespace));

        assert!(policy.key_authorized_for_package(&namespace_key, &my_package));
        assert!(policy.key_authorized_for_package(&namespace_key, &my_namespace_other_package));
        assert!(!policy.key_authorized_for_package(&namespace_key, &other_namespace));

        assert!(policy.key_authorized_for_package(&package_key, &my_package));
        assert!(!policy.key_authorized_for_package(&package_key, &my_namespace_other_package));
        assert!(!policy.key_authorized_for_package(&package_key, &other_namespace));

        assert!(!policy.key_authorized_for_package(&other_key, &my_package));
        assert!(!policy.key_authorized_for_package(&other_key, &my_namespace_other_package));
        assert!(!policy.key_authorized_for_package(&other_key, &other_namespace));

        Ok(())
    }
}
