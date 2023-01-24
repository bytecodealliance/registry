use anyhow::Result;
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, DirEntry},
    path::Path,
};
use warg_crypto::hash::HashAlgorithm;
use warg_crypto::signing;
use warg_protocol::{
    operator::{self, Validator},
    protobuf, ProtoEnvelope,
};

#[test]
fn test_operator_logs() {
    let mut entries: Vec<DirEntry> = fs::read_dir("./tests/operator-logs")
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    entries.sort_by_key(|e| e.file_name());

    fs::create_dir_all("./tests/operator-logs/output").unwrap();

    for entry in entries {
        if entry.metadata().unwrap().is_file() {
            execute_test(&entry.path());
        }
    }
}

fn validate_input(input: Vec<EnvelopeData>) -> Result<Validator> {
    input
        .into_iter()
        .scan(None, |last, e_data| {
            let key: signing::PrivateKey = e_data.key.parse().unwrap();
            let mut record: operator::OperatorRecord = e_data.contents.try_into().unwrap();

            record.prev = last.clone();

            let envelope = ProtoEnvelope::signed_contents(&key, record).unwrap();

            *last = Some(HashAlgorithm::Sha256.digest(envelope.content_bytes()));

            Some(envelope)
        })
        .try_fold(Validator::new(), |mut validator, record| {
            validator.validate(&record)?;
            Ok(validator)
        })
}

fn execute_test(input_path: &Path) {
    let output_path = Path::new("./tests/operator-logs/output").join(
        input_path
            .file_name()
            .expect("expected a file name for test input"),
    );
    let input: Vec<EnvelopeData> = serde_json::from_str(
        &fs::read_to_string(input_path)
            .map_err(|e| {
                format!(
                    "failed to read input file `{path}`: {e}",
                    path = input_path.display()
                )
            })
            .unwrap(),
    )
    .map_err(|e| {
        format!(
            "failed to deserialize input file `{path}`: {e}",
            path = input_path.display()
        )
    })
    .unwrap();

    let output = match validate_input(input) {
        Ok(validator) => Output::Valid(validator),
        Err(e) => Output::Error(e.to_string()),
    };

    if std::env::var_os("BLESS").is_some() {
        // Update the test baseline
        fs::write(&output_path, serde_json::to_string_pretty(&output).unwrap())
            .map_err(|e| {
                format!(
                    "failed to write output file `{path}`: {e}",
                    path = output_path.display()
                )
            })
            .unwrap();
    } else {
        assert_eq!(
            serde_json::from_str::<Output>(
                &fs::read_to_string(&output_path)
                    .map_err(|e| {
                        format!(
                            "failed to read output file `{path}`: {e}",
                            path = output_path.display()
                        )
                    })
                    .unwrap()
            )
            .map_err(|e| format!(
                "failed to deserialize output file `{path}`: {e}",
                path = output_path.display()
            ))
            .unwrap(),
            output
        );
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeData {
    key: String,
    contents: protobuf::OperatorRecord,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Output {
    Valid(Validator),
    Error(String),
}
