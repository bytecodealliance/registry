use std::fs::{self, DirEntry};

use hashbrown::HashMap;
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use serde_json;
use warg_protocol::{
    hash,
    operator::{self, validate::ValidationState},
    protobuf, signing, Envelope,
};

#[test]
fn test_operator_logs() {
    let mut entries: Vec<DirEntry> = fs::read_dir("./tests/operator-logs")
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
    let envelopes: Vec<Envelope<operator::model::OperatorRecord>> = test
        .input
        .into_iter()
        .scan(None, |last, e_data| {
            dbg!(e_data.contents.clone());
            let key: signing::PrivateKey = e_data.key.parse().unwrap();
            let mut record: operator::model::OperatorRecord = e_data.contents.try_into().unwrap();

            record.prev = last.clone();

            let envelope = Envelope::signed_contents(&key, record).unwrap();

            *last = Some(hash::HashAlgorithm::Sha256.digest(envelope.content_bytes()));

            Some(envelope)
        })
        .collect();

    let mut validation_state = Ok(operator::validate::ValidationState::Uninitialized);

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
        Err(error) => OperatorStateSummary::Invalid {
            error: format!("{}", error),
        },
    };

    assert_eq!(test.output, result);
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Test {
    input: Vec<EnvelopeData>,
    output: OperatorStateSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeData {
    key: String,
    contents: protobuf::OperatorRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatorStateSummary {
    Valid {
        hash_algorithm: String,
        permissions: HashMap<String, Vec<String>>,
    },
    Invalid {
        error: String,
    },
}

impl From<operator::validate::EntryValidationState> for OperatorStateSummary {
    fn from(state: operator::validate::EntryValidationState) -> Self {
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

        OperatorStateSummary::Valid {
            hash_algorithm: format!("{}", state.hash_algorithm),
            permissions,
        }
    }
}
