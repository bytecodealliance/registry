use anyhow::{anyhow, Context, Result};
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, DirEntry},
    path::Path,
};
use warg_protocol::{
    hash,
    package::{self, validate::Validator},
    protobuf, signing, Envelope,
};

#[test]
fn test_package_logs() -> Result<()> {
    let mut entries: Vec<DirEntry> =
        fs::read_dir("./tests/package-logs")?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());

    fs::create_dir_all("./tests/package-logs/output")?;

    for entry in entries {
        if entry.metadata()?.is_file() {
            execute_test(&entry.path())?;
        }
    }

    Ok(())
}

fn validate_input(input: Vec<EnvelopeData>) -> Result<Validator> {
    let mut validator = Validator::new();
    for record in input.into_iter().scan(None, |last, e_data| {
        let key: signing::PrivateKey = e_data.key.parse().unwrap();
        let mut record: package::model::PackageRecord = e_data.contents.try_into().unwrap();

        record.prev = last.clone();

        let envelope = Envelope::signed_contents(&key, record).unwrap();

        *last = Some(hash::HashAlgorithm::Sha256.digest(envelope.content_bytes()));

        Some(envelope)
    }) {
        validator.validate(&record)?;
    }

    Ok(validator)
}

fn execute_test(input_path: &Path) -> Result<()> {
    let output_path = Path::new("./tests/package-logs/output").join(
        input_path
            .file_name()
            .ok_or_else(|| anyhow!("expected a file name for test input"))?,
    );
    let input: Vec<EnvelopeData> =
        serde_json::from_str(&fs::read_to_string(input_path).with_context(|| {
            format!(
                "failed to read input file `{path}`",
                path = input_path.display()
            )
        })?)
        .with_context(|| {
            format!(
                "failed to deserialize input file `{path}`",
                path = input_path.display()
            )
        })?;

    let output = match validate_input(input) {
        Ok(validator) => Output::Valid(validator),
        Err(e) => Output::Error(e.to_string()),
    };

    if std::env::var_os("BLESS").is_some() {
        // Update the test baseline
        fs::write(&output_path, serde_json::to_string_pretty(&output)?).with_context(|| {
            format!(
                "failed to write output file `{path}`",
                path = output_path.display()
            )
        })?;
    } else {
        assert_eq!(
            serde_json::from_str::<Output>(&fs::read_to_string(&output_path).with_context(
                || {
                    format!(
                        "failed to read output file `{path}`",
                        path = output_path.display()
                    )
                }
            )?)
            .with_context(|| format!(
                "failed to deserialize output file `{path}`",
                path = output_path.display()
            ))?,
            output
        );
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeData {
    key: String,
    contents: protobuf::PackageRecord,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Output {
    Valid(Validator),
    Error(String),
}
