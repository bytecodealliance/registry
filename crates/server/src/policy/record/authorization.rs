use super::{RecordPolicy, RecordPolicyError, RecordPolicyResult};
use anyhow::{bail, Result};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use warg_crypto::signing::KeyID;
use warg_protocol::{
    package::{PackageEntry, PackageRecord},
    registry::PackageId,
    ProtoEnvelope,
};
use wasmparser::names::KebabStr;

/// A policy that ensures a published record is signed by an authorized key.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizedKeyPolicy {
    #[serde(skip)]
    superuser_keys: HashSet<KeyID>,
    #[serde(default, rename = "namespace")]
    namespaces: HashMap<String, LogPolicy>,
    #[serde(default, rename = "package")]
    packages: HashMap<PackageId, LogPolicy>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogPolicy {
    // Authorized key IDs
    keys: HashSet<KeyID>,
    // If true, permission grants are permitted.
    #[serde(default)]
    delegation: bool,
}

impl LogPolicy {
    fn key_authorized_for_entry(&self, key: &KeyID, is_init: bool) -> bool {
        (self.delegation && !is_init) || self.keys.contains(key)
    }
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
        self.namespace_or_default_mut(namespace)?.keys.insert(key);
        Ok(self)
    }

    /// Enables delegation for a particular namespace.
    pub fn with_namespace_delegation(mut self, namespace: impl Into<String>) -> Result<Self> {
        self.namespace_or_default_mut(namespace)?.delegation = true;
        Ok(self)
    }

    fn namespace_or_default_mut(&mut self, namespace: impl Into<String>) -> Result<&mut LogPolicy> {
        let namespace = namespace.into();
        if KebabStr::new(&namespace).is_none() {
            bail!("namespace `{namespace}` is not a legal kebab-case identifier");
        }

        Ok(self.namespaces.entry(namespace).or_default())
    }

    /// Sets an authorized key for publishing to a particular package.
    pub fn with_package_key(mut self, package_id: impl Into<String>, key: KeyID) -> Result<Self> {
        self.package_or_default_mut(package_id)?.keys.insert(key);
        Ok(self)
    }

    /// Enables delegation for a particular package.
    pub fn with_package_delegation(mut self, package_id: impl Into<String>) -> Result<Self> {
        self.package_or_default_mut(package_id)?.delegation = true;
        Ok(self)
    }

    fn package_or_default_mut(&mut self, package_id: impl Into<String>) -> Result<&mut LogPolicy> {
        let package_id = PackageId::new(package_id)?;
        Ok(self.packages.entry(package_id).or_default())
    }

    pub fn key_authorized_for_entry(
        &self,
        key: &KeyID,
        package: &PackageId,
        is_init: bool,
    ) -> bool {
        if self.superuser_keys.contains(key) {
            return true;
        }

        if let Some(policy) = self.namespaces.get(package.namespace()) {
            if policy.key_authorized_for_entry(key, is_init) {
                return true;
            }
        }

        if let Some(policy) = self.packages.get(package) {
            if policy.key_authorized_for_entry(key, is_init) {
                return true;
            }
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
        for entry in &record.as_ref().entries {
            let is_init = matches!(entry, PackageEntry::Init { .. });
            if !self.key_authorized_for_entry(key, id, is_init) {
                return Err(RecordPolicyError::Unauthorized(format!(
                    "key id `{key}` is not authorized to publish to package `{id}`",
                )));
            }
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
        let my_ns_other_package: PackageId = "my-namespace:other-package".parse()?;
        let other_namespace: PackageId = "other-namespace:any-package".parse()?;

        assert!(policy.key_authorized_for_entry(&super_key, &my_package, false));
        assert!(policy.key_authorized_for_entry(&super_key, &my_ns_other_package, false));
        assert!(policy.key_authorized_for_entry(&super_key, &other_namespace, false));

        assert!(policy.key_authorized_for_entry(&namespace_key, &my_package, false));
        assert!(policy.key_authorized_for_entry(&namespace_key, &my_ns_other_package, false));
        assert!(!policy.key_authorized_for_entry(&namespace_key, &other_namespace, false));

        assert!(policy.key_authorized_for_entry(&package_key, &my_package, false));
        assert!(!policy.key_authorized_for_entry(&package_key, &my_ns_other_package, false));
        assert!(!policy.key_authorized_for_entry(&package_key, &other_namespace, false));

        assert!(!policy.key_authorized_for_entry(&other_key, &my_package, false));
        assert!(!policy.key_authorized_for_entry(&other_key, &my_ns_other_package, false));
        assert!(!policy.key_authorized_for_entry(&other_key, &other_namespace, false));

        Ok(())
    }

    #[test]
    fn test_key_authorized_for_package_init() -> Result<()> {
        let authed_key = KeyID::from("authed-key".to_string());
        let other_key = KeyID::from("other-key".to_string());

        let policy = AuthorizedKeyPolicy::new()
            .with_namespace_key("ns1", authed_key.clone())?
            .with_namespace_delegation("ns1")?
            .with_package_key("ns2:pkg", authed_key.clone())?
            .with_package_delegation("ns2:pkg")?;

        let ns1_pkg: PackageId = "ns1:pkg".parse()?;
        let ns2_pkg: PackageId = "ns2:pkg".parse()?;

        assert!(policy.key_authorized_for_entry(&authed_key, &ns1_pkg, true));
        assert!(policy.key_authorized_for_entry(&authed_key, &ns1_pkg, false));
        assert!(policy.key_authorized_for_entry(&authed_key, &ns2_pkg, true));
        assert!(policy.key_authorized_for_entry(&authed_key, &ns2_pkg, false));

        assert!(!policy.key_authorized_for_entry(&other_key, &ns1_pkg, true));
        assert!(policy.key_authorized_for_entry(&other_key, &ns1_pkg, false));
        assert!(!policy.key_authorized_for_entry(&other_key, &ns2_pkg, true));
        assert!(policy.key_authorized_for_entry(&other_key, &ns2_pkg, false));
        Ok(())
    }
}
