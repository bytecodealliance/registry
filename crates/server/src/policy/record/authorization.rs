use super::{RecordPolicy, RecordPolicyError, RecordPolicyResult};
use anyhow::{bail, Result};
use std::collections::{HashMap, HashSet};
use warg_crypto::signing::KeyID;
use warg_protocol::{package::PackageRecord, registry::PackageId, ProtoEnvelope};
use wasmparser::names::KebabStr;

/// A policy that ensures a published record is authorized by
/// the key signing the record.
#[derive(Default)]
pub struct AuthorizedKeyPolicy {
    keys: HashSet<KeyID>,
    namespaces: HashMap<String, HashSet<KeyID>>,
}

impl AuthorizedKeyPolicy {
    /// Creates a new authorized key policy.
    ///
    /// By default, no keys are authorized.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets an authorized key for publishing to any namespace.
    pub fn with_key(mut self, key: KeyID) -> Self {
        self.keys.insert(key);
        self
    }

    /// Sets an authorized key for publishing to a particular namespace.
    pub fn with_namespace_key(mut self, namespace: impl Into<String>, key: KeyID) -> Result<Self> {
        let namespace = namespace.into();
        if KebabStr::new(&namespace).is_none() {
            bail!("namespace `{namespace}` is not a legal kebab-case identifier");
        }

        self.namespaces.entry(namespace).or_default().insert(key);
        Ok(self)
    }
}

impl RecordPolicy for AuthorizedKeyPolicy {
    fn check(
        &self,
        id: &PackageId,
        record: &ProtoEnvelope<PackageRecord>,
    ) -> RecordPolicyResult<()> {
        if !self.keys.contains(record.key_id())
            && !self
                .namespaces
                .get(id.namespace())
                .map(|keys| keys.contains(record.key_id()))
                .unwrap_or(false)
        {
            return Err(RecordPolicyError::Unauthorized(format!(
                "key id `{key}` is not authorized to publish to package `{id}`",
                key = record.key_id()
            )));
        }

        Ok(())
    }
}
