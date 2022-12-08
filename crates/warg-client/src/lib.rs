use semver::Version;
use url::Url;

/// A client for interacting with a registry
#[allow(dead_code)]
pub struct Client<RegistryCache, ValidationCache> {
    registry: DomainName,
    known_roots: Vec<Root>,
    registry_cache: RegistryCache,
    validation_cache: ValidationCache,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainName(String);

#[allow(dead_code)]
pub struct Root {
    bytes: Vec<u8>
}

pub trait Cache {
    /// Insert a value into the clients cache.
    fn insert(&mut self, key: Vec<u8>, value: Vec<u8>);

    /// Lookup a previously inserted cache value.
    fn lookup(&self, key: &[u8]) -> Option<Vec<u8>>;
}

impl<RegistryCache, ValidationCache> Client<RegistryCache, ValidationCache>
where
    RegistryCache: Cache,
    ValidationCache: Cache
{
    /// Create a new client with no previous state
    pub fn new(registry: DomainName, registry_cache: RegistryCache, validation_cache: ValidationCache) -> Self {
        Self {
            registry,
            known_roots: Vec::new(),
            registry_cache,
            validation_cache
        }
    }

    /// Check what registry is being used
    pub fn registry(&self) -> DomainName {
        self.registry.clone()
    }

    /// Serialize the client state
    pub fn to_state(&self) -> Vec<u8> {
        unimplemented!()
    }

    /// Construct a registry client with prior state
    pub fn from_state(registry: DomainName, state: Vec<u8>, registry_cache: RegistryCache, validation_cache: ValidationCache) -> Self {
        drop(state);
        Self {
            registry,
            known_roots: Vec::new(),
            registry_cache,
            validation_cache
        };
        todo!("Deserialize known roots from state");
    }

    /// Retrieve the current valid root
    pub fn root(&self) -> Option<Root> {
        unimplemented!()
    }

    /// Return the information for a package
    pub fn package_info(&mut self, root: Root, package: String) -> PackageInfo {
        drop(root);
        drop(package);
        unimplemented!()
    }
}

/// Info about a package at a point in time
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageInfo {
    pub name: String,
    /// What kind of package this is
    pub package_type: PackageType,
    /// The available releases of the package.
    /// Does not include yanked releases.
    pub releases: Vec<ReleaseInfo>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageType {
    Component,
    Module,
    Interface,
    World
}

/// Info about an available package release
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    /// The version of the release
    pub version: Version,
    /// The release content digest
    pub digest: Vec<u8>,
    /// The known locations where the content may be found
    pub known_locations: Vec<Location>
}

/// A way of retrieving package contents
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Location {
    HTTP(Url)
}