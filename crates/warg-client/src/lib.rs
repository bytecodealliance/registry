use std::collections::HashMap;

struct Client<Cache> {
    cache: Cache
}

struct Domain(String);

impl Client {
    pub fn new(primary_registry: Domain) -> Self {
        todo!()
    }

    pub fn from_state(primary_registry: Domain, serialized_state: Vec<u8>) -> Self {
        todo!()
    }

    pub fn to_state(&self) -> Vec<u8> {
        todo!()
    }

    pub fn query(&mut self, constraints: VersionConstraints) -> VersionConstraints {
        todo!()
    }

    pub fn lookup(&mut self, version_spec: VersionSpec) -> Location {
        todo!()
    }
}

struct VersionConstraints {
    packages: HashMap<String, VersionRequirement>
}

enum VersionRequirement {
    LessThan(Version),
    GreaterThan(Version)
}

struct Version {
    major: u32,
    minor: u32,
    patch: u32
}

struct VersionSet {
    packages: HashMap<String, VersionSpec>
}

struct VersionSpec {
    package: String,
    version: Version,
    digest: String
}

enum Location {
    HTTP(String)
}