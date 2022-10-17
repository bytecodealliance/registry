use prost::Message;

use crate::things::envelope::{Envelope, WithBytes};

pub mod model;
pub mod validate;

pub mod protobuf {
    include!(concat!(env!("OUT_DIR"), "/warg.package.rs"));
}

pub fn parse_package_entry(bytes: Vec<u8>) -> Result<WithBytes<model::Entry>, ()> {
    let proto_entry = protobuf::Entry::decode(&*bytes).map_err(|_| ())?;

    let contents = match proto_entry.contents.ok_or(())? {
        protobuf::entry::Contents::Init(init) => model::Entry::Init {
            hash_algorithm: init.hash_algo.parse()?,
            key: init.key.parse()?,
        },
        protobuf::entry::Contents::UpdateAuth(update_auth) => model::Entry::UpdateAuth {
            key: update_auth.key.parse()?,
            allow: parse_permission_list(update_auth.allow)?,
            deny: parse_permission_list(update_auth.deny)?,
        },
        protobuf::entry::Contents::Release(release) => model::Entry::Release {
            version: release.version.parse().map_err(|_| ())?,
            content: release.content_hash.parse()?,
        },
        protobuf::entry::Contents::Yank(yank) => model::Entry::Yank {
            version: yank.version.parse().map_err(|_| ())?,
        },
    };

    Ok(WithBytes { contents, bytes })
}

fn parse_permission_list(
    p_list: Option<protobuf::PermissionList>,
) -> Result<Vec<model::Permission>, ()> {
    p_list
        .ok_or(())?
        .permissions
        .into_iter()
        .map(|p_string| p_string.parse::<model::Permission>())
        .collect()
}

impl model::Entry {
    pub fn with_bytes(self) -> WithBytes<model::Entry> {
        let proto_entry = package_entry_to_protobuf(&self);
        let mut bytes: Vec<u8> = Vec::new();
        proto_entry.encode(&mut bytes).unwrap();

        WithBytes {
            contents: self,
            bytes,
        }
    }
}

fn package_entry_to_protobuf(entry: &model::Entry) -> protobuf::Entry {
    let contents = match entry {
        model::Entry::Init {
            hash_algorithm: hash_algo,
            key,
        } => protobuf::entry::Contents::Init(protobuf::Init {
            key: key.to_string(),
            hash_algo: hash_algo.to_string(),
        }),
        model::Entry::UpdateAuth { key, allow, deny } => {
            protobuf::entry::Contents::UpdateAuth(protobuf::UpdateAuth {
                key: key.to_string(),
                allow: permission_list_to_protobuf(allow),
                deny: permission_list_to_protobuf(deny),
            })
        }
        model::Entry::Release {
            version,
            content: content_hash,
        } => protobuf::entry::Contents::Release(protobuf::Release {
            version: version.to_string(),
            content_hash: content_hash.to_string(),
        }),
        model::Entry::Yank { version } => protobuf::entry::Contents::Yank(protobuf::Yank {
            version: version.to_string(),
        }),
    };

    protobuf::Entry {
        contents: Some(contents),
    }
}

fn permission_list_to_protobuf(
    permissions: &Vec<model::Permission>,
) -> Option<protobuf::PermissionList> {
    Some(protobuf::PermissionList {
        permissions: permissions.iter().map(|p| p.to_string()).collect(),
    })
}
