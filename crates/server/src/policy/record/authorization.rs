use super::{RecordPolicy, RecordPolicyError, RecordPolicyResult};
use crate::is_kebab_case;
use anyhow::{bail, Result};
use std::collections::{HashMap, HashSet};
use warg_crypto::signing::KeyID;
use warg_protocol::{package::PackageRecord, registry::PackageId, ProtoEnvelope};

/// A policy that ensures a published record is authorized by
/// the key signing the record.
#[derive(Default)]
pub struct AuthorizedKeyPolicy {
    namespaces: HashMap<String, HashSet<KeyID>>,
}

impl AuthorizedKeyPolicy {
    /// Creates a new authorized key policy.
    ///
    /// By default, no keys are authorized.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets an authorized key for a particular namespace.
    pub fn with_authorized_key(mut self, namespace: impl Into<String>, key: KeyID) -> Result<Self> {
        let namespace = namespace.into();
        if !is_kebab_case(&namespace) {
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
        if !self
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
