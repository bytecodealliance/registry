use std::fs::{self, DirEntry};

use hashbrown::HashMap;
use pretty_assertions::assert_eq;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json;
use warg_protocol::{
    hash,
    package::{self, validate::ValidationState},
    protobuf, signing, Envelope,
};

#[test]
fn test_package_logs() {
    let mut entries: Vec<DirEntry> = fs::read_dir("./tests/package-logs")
        .unwrap()
        .collect::<Result<Vec<DirEntry>, _>>()
        .unwrap();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let file_contents = std::fs::read_to_string(entry.path()).unwrap();
        let test: Test = serde_json::from_str(&file_contents).unwrap();

        execute_test(test);
    }
}

fn execute_test(test: Test) {
    let envelopes: Vec<Envelope<package::model::PackageRecord>> = test
        .input
        .into_iter()
        .scan(None, |last, e_data| {
            dbg!(e_data.contents.clone());
            let key: signing::PrivateKey = e_data.key.parse().unwrap();
            let mut record: package::model::PackageRecord = e_data.contents.try_into().unwrap();

            record.prev = last.clone();

            let envelope = Envelope::signed_contents(&key, record).unwrap();

            *last = Some(hash::HashAlgorithm::Sha256.digest(&envelope.content_bytes));

            Some(envelope)
        })
        .collect();

    let mut validation_state = Ok(package::validate::ValidationState::Uninitialized);

    for envelope in envelopes {
        if matches!(validation_state, Err(_)) {
            break;
        }

        validation_state = validation_state.unwrap().validate_envelope(&envelope);
    }

    let result = match validation_state {
        Ok(state) => {
            if let ValidationState::Initialized(state) = state {
                state.entry_state.into()
            } else {
                panic!("Test did not initialize state. Test input must not be empty.");
            }
        }
        Err(error) => PackageStateSummary::Invalid {
            error: format!("{}", error),
        },
    };

    assert_eq!(test.output, result);
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Test {
    input: Vec<EnvelopeData>,
    output: PackageStateSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeData {
    key: String,
    contents: protobuf::PackageRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageStateSummary {
    Valid {
        hash_algorithm: String,
        permissions: HashMap<String, Vec<String>>,
        releases: HashMap<String, ReleaseStateSummary>,
    },
    Invalid {
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReleaseStateSummary {
    Unreleased,
    Released { content: String },
    Yanked,
}

impl From<package::validate::EntryValidationState> for PackageStateSummary {
    fn from(state: package::validate::EntryValidationState) -> Self {
        let permissions = state
            .permissions
            .into_iter()
            .map(|(k, v)| {
                let mut vec_perms: Vec<String> = v
                    .into_iter()
                    .map(|permission| format!("{}", permission))
                    .collect();
                vec_perms.sort();
                (format!("{}", k), vec_perms)
            })
            .collect();

        let mut releases: Vec<(Version, package::validate::ReleaseState)> =
            state.releases.into_iter().collect();

        releases.sort_by_key(|(v, _s)| v.clone());

        let releases = releases
            .into_iter()
            .map(|(k, v)| (format!("{}", k), v.into()))
            .collect();

        PackageStateSummary::Valid {
            hash_algorithm: format!("{}", state.hash_algorithm),
            permissions,
            releases,
        }
    }
}

impl From<package::validate::ReleaseState> for ReleaseStateSummary {
    fn from(release_state: package::validate::ReleaseState) -> Self {
        match release_state {
            package::validate::ReleaseState::Unreleased => ReleaseStateSummary::Unreleased,
            package::validate::ReleaseState::Released { content } => {
                ReleaseStateSummary::Released {
                    content: format!("{}", content),
                }
            }
            package::validate::ReleaseState::Yanked => ReleaseStateSummary::Yanked,
        }
    }
}
