use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashSet;
use warg_crypto::{hash::AnyHash, Decode};

pub mod operator;
pub mod package;
pub mod proto_envelope;
pub mod registry;
mod serde_envelope;

pub use proto_envelope::{ProtoEnvelope, ProtoEnvelopeBody};
pub use semver::{Version, VersionReq};
pub use serde_envelope::SerdeEnvelope;

/// Trait implemented by the record types.
pub trait Record: Clone + Decode + Send + Sync {
    /// Gets the set of content hashes associated with the record.
    ///
    /// An empty set indicates that the record has no associated content.
    fn contents(&self) -> HashSet<&AnyHash>;
}

/// Trait implemented by the validator types.
pub trait Validator:
    std::fmt::Debug + Serialize + DeserializeOwned + Default + Send + Sync
{
    /// The type of record being validated.
    type Record: Record;

    /// The type of error returned when validation fails.
    type Error: Send;

    /// Validates the given record.
    fn validate(&mut self, record: &ProtoEnvelope<Self::Record>) -> Result<(), Self::Error>;
}

/// Helpers for converting to and from protobuf

fn prost_to_pbjson_timestamp(timestamp: prost_types::Timestamp) -> pbjson_types::Timestamp {
    pbjson_types::Timestamp {
        seconds: timestamp.seconds,
        nanos: timestamp.nanos,
    }
}

fn pbjson_to_prost_timestamp(timestamp: pbjson_types::Timestamp) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: timestamp.seconds,
        nanos: timestamp.nanos,
    }
}

/// Helper module for serializing and deserializing timestamps.
///
/// This is used over serde's built-in implementation to produce cleaner timestamps
/// in serialized output.
mod timestamp {
    use serde::Deserializer;
    use serde::{Deserialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(timestamp: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;

        let duration_since_epoch = match timestamp.duration_since(UNIX_EPOCH) {
            Ok(duration_since_epoch) => duration_since_epoch,
            Err(_) => return Err(S::Error::custom("timestamp must be later than UNIX_EPOCH")),
        };

        serializer.serialize_str(&format!(
            "{secs}.{nsecs}",
            secs = duration_since_epoch.as_secs(),
            nsecs = duration_since_epoch.subsec_nanos()
        ))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        let s = String::deserialize(deserializer)?;
        let (secs, nsecs) = s
            .split_once('.')
            .ok_or_else(|| D::Error::custom("timestamp must be in the format <secs>.<nsecs>"))?;

        Ok(SystemTime::UNIX_EPOCH
            + Duration::new(
                secs.parse::<u64>().map_err(D::Error::custom)?,
                nsecs.parse::<u32>().map_err(D::Error::custom)?,
            ))
    }
}
