use std::{fmt, time::SystemTime};

use serde::{Deserialize, Serialize};
use warg_crypto::{
    hash::{DynHash, HashAlgorithm},
    signing,
};
use warg_protocol::{package, Version};

#[derive(Serialize, Deserialize)]
pub struct PublishInfo {
    package: String,
    prev: Option<DynHash>,
    entries: Vec<PackageEntryInfo>,
}

impl PublishInfo {
    pub fn new(package: String, prev: Option<DynHash>) -> Self {
        Self {
            package,
            prev,
            entries: vec![],
        }
    }

    pub fn package(&self) -> &str {
        &self.package
    }

    pub fn push_init(&mut self, hash_algorithm: HashAlgorithm, key: signing::PublicKey) {
        let init = PackageEntryInfo::Init {
            hash_algorithm,
            key,
        };
        self.entries.push(init);
    }

    pub fn push_release(&mut self, version: Version, content: DynHash) {
        let release = PackageEntryInfo::Release { version, content };
        self.entries.push(release);
    }

    pub fn finalize(self) -> (String, Vec<DynHash>, package::PackageRecord) {
        let name = self.package;
        let content = self
            .entries
            .iter()
            .filter_map(|entry| match entry {
                PackageEntryInfo::Init { .. } => None,
                PackageEntryInfo::Release { content, .. } => Some(content.to_owned()),
            })
            .collect();
        let package = package::PackageRecord {
            prev: self.prev,
            version: package::PACKAGE_RECORD_VERSION,
            timestamp: SystemTime::now(),
            entries: self
                .entries
                .into_iter()
                .map(|entry| match entry {
                    PackageEntryInfo::Init {
                        hash_algorithm,
                        key,
                    } => package::PackageEntry::Init {
                        hash_algorithm,
                        key,
                    },
                    PackageEntryInfo::Release { version, content } => {
                        package::PackageEntry::Release { version, content }
                    }
                })
                .collect(),
        };
        (name, content, package)
    }
}

impl fmt::Display for PublishInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Publishing package: {} ({} entries)\n",
            self.package,
            self.entries.len()
        )?;
        if let Some(prev) = &self.prev {
            println!("(Previous record hash: {})", prev);
        } else {
            println!("(No previous record, this publish must init)");
        }
        for (i, entry) in self.entries.iter().enumerate() {
            write!(f, "{}. {}\n", i, &entry)?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub enum PackageEntryInfo {
    Init {
        hash_algorithm: HashAlgorithm,
        key: signing::PublicKey,
    },
    Release {
        version: Version,
        content: DynHash,
    },
}

impl fmt::Display for PackageEntryInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageEntryInfo::Init {
                hash_algorithm,
                key,
            } => {
                write!(f, "Init {} - {}", hash_algorithm, key)
            }
            PackageEntryInfo::Release { version, content } => {
                write!(f, "Release {} - {}", version, content)
            }
        }
    }
}
