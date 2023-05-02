#[cfg(test)]
mod tests {
    use crate::pb;

    #[test]
    fn package_record_serialize() {
        let msg = pb::PackageRecord {
            prev: None,
            version: 123,
            time: Some(prost_wkt_types::Timestamp {
                seconds: 1683050490,
                nanos: 0,
            }),
            entries: vec![pb::PackageEntry {
                contents: Some(pb::package_entry::Contents::Init(pb::PackageInit {
                    key: String::from("hello-world"),
                    hash_algorithm: String::from("sha256"),
                })),
            }],
        };

        let serialized = serde_json::to_string(&msg).unwrap();
        assert_eq!(
            serialized,
            r#"{"prev":null,"version":123,"time":"2023-05-02T18:01:30Z","entries":[{"contents":{"init":{"key":"hello-world","hashAlgorithm":"sha256"}}}]}"#
        );
    }
}
