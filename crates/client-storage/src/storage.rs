use std::{fs, path::{Path, PathBuf}, io::Write};
use anyhow::{Context, Error, Result};
use serde::{Deserialize, Serialize};
use tempfile::{NamedTempFile, TempPath};
use warg_protocol::{
  package,
  registry::{MapCheckpoint, RecordId},
  SerdeEnvelope, Version,
};
use warg_crypto::{
  hash::{Digest, Hash, DynHash, HashAlgorithm, Sha256},
  signing,
};

struct ExpectedFileContent<'storage> {
  expected_hash: DynHash,
  storage: &'storage mut FileSystemStorage,
  hasher: Sha256,
  tmp_path: TempPath,
  file: fs::File,
}

impl<'storage> ExpectedContent for ExpectedFileContent<'storage> {
  fn write_all(&mut self, bytes: &[u8]) -> Result<()> {
      self.hasher.update(bytes);
      self.file.write_all(bytes);
      Ok(())
  }

  fn finalize(self: Box<Self>) -> Result<()> {
      let hash = self.hasher.finalize();
      let hash: Hash<Sha256> = hash.into();
      let hash: DynHash = hash.into();
      if hash == self.expected_hash {
          if !self.storage.has_content(&self.expected_hash)? {
              let path = self.storage.content_path(&self.expected_hash)?;
              self.tmp_path.persist(path)?;
          }
          Ok(())
      } else {
          Err(Error::msg("Downloaded content digest did not match"))
      }
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

#[derive(Serialize, Deserialize)]
pub struct PublishInfo {
    pub package: String,
    pub prev: Option<RecordId>,
    pub entries: Vec<PackageEntryInfo>,
}

pub trait ExpectedContent {
  /// Write new bytes of the content
  fn write_all(&mut self, bytes: &[u8]) -> Result<()>;

  /// Finalize the content, storing it if the hash matched the expected value
  /// and returning an error if it didn't.
  fn finalize(self: Box<Self>) -> Result<()>;
}
pub trait NewContent {
    fn write_all(&mut self, bytes: &[u8]) -> Result<()>;

    fn finalize(self: Box<Self>) -> Result<DynHash>;
}

pub struct FileSystemStorage {
  base: PathBuf,
  publish_info: PathBuf,
  registry_info: PathBuf,
}

impl FileSystemStorage {
  pub fn new(base: impl Into<PathBuf>) -> Result<Self> {
    let base = base.into();
    fs::create_dir_all(&base).with_context(|| {
        format!("failed to create directory `{base}`", base = base.display())
    })?;

    let publish_info = base.join("publish-info.json");
    let registry_info = base.join("registry-info.json");

    Ok(Self {
        base,
        publish_info,
        registry_info,
    })
  }

  pub fn temp_dir(&self) -> Result<PathBuf> {
    let path = self.base.join("temp");

    fs::create_dir_all(&path).with_context(|| {
        format!("failed to create directory `{path}`", path = path.display())
    })?;

    Ok(path)
  }

  fn content_dir(&self) -> Result<PathBuf> {
    let path = self.base.join("content");

    fs::create_dir_all(&path).with_context(|| {
        format!("failed to create directory `{path}`", path = path.display())
    })?;

    Ok(path)
  }

  pub fn content_path(&self, digest: &DynHash) -> Result<PathBuf> {
    let mut path = self.content_dir()?;
    path.push(digest.to_string().replace(':', "-"));
    Ok(path)
  }

  fn package_dir(&self) -> Result<PathBuf> {
    let path = self.base.join("package");

    fs::create_dir_all(&path).with_context(|| {
        format!("failed to create directory `{path}`", path = path.display())
    })?;

    Ok(path)
  }

  fn package_path(&self, name: &str) -> Result<PathBuf> {
    let mut path = self.package_dir()?;
    path.push(name);
    Ok(path)
  }
}

#[derive(Serialize, Deserialize)]
pub struct RegistryInfo {
    pub url: String,
    pub checkpoint: Option<SerdeEnvelope<MapCheckpoint>>,
}

pub trait ClientStorage {
  fn load_registry_info(&self) -> Result<Option<RegistryInfo>>;

  fn store_registry_info(&mut self, info: &RegistryInfo) -> Result<()>;

  fn load_publish_info(&self) -> Result<Option<PublishInfo>>;

  fn store_publish_info(&mut self, info: &PublishInfo) -> Result<()>;

  fn clear_publish_info(&mut self) -> Result<()>;

  fn list_all_packages(&self) -> Result<Vec<String>>;

  fn load_package_state(&self, package: &str) -> Result<package::Validator>;

  fn store_package_state(
      &mut self,
      package: &str,
      state: &package::Validator,
  ) -> Result<()>;

  fn has_content(&self, digest: &DynHash) -> Result<bool>;

  fn store_content<'s>(
      &'s mut self,
      digest: DynHash,
  ) -> Result<Box<dyn ExpectedContent + 's>>;

  // fn create_content<'s>(&'s mut self) -> Result<Box<dyn NewContent + 's>>;

  // fn get_content(&self, digest: &DynHash) -> Result<Option<Vec<u8>>>;
}

async fn dump(path: &Path, contents: &[u8]) -> Result<()> {
  if path.exists() {
      std::fs::remove_file(path)?;
  }
  if let Some(parent) = path.parent() {
      if parent.is_file() {
          return Err(Error::msg("Parent directory is file"));
      }
      if !parent.exists() {
          std::fs::create_dir_all(parent)?;
      }
  }
  std::fs::write(path, contents)?;
  Ok(())
}

fn delete(path: &Path) -> Result<()> {
  std::fs::remove_file(path)?;
  Ok(())
}

fn load<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<Option<T>> {
  if path.is_file() {
      let contents = std::fs::read_to_string(path)?;
      serde_json::from_str(&contents)
          .map_err(|_| Error::msg(format!("Info at path {:?} is malformed", &path)))
  } else {
      Ok(None)
  }
}

fn load_or_default<T: Default + for<'a> Deserialize<'a>>(path: &Path) -> Result<T> {
  if path.is_file() {
      let contents = std::fs::read_to_string(path)?;
      serde_json::from_str(&contents)
          .map_err(|_| Error::msg(format!("Info at path {:?} is malformed", &path)))
  } else {
      Ok(Default::default())
  }
}

fn store<T: Serialize>(path: &Path, value: &T) -> Result<()> {
  let contents = serde_json::to_vec_pretty(value)?;
  dump(path, &contents);
  Ok(())
}

impl ClientStorage for FileSystemStorage {
  fn load_registry_info(&self) -> Result<Option<RegistryInfo>> {
      load(&self.registry_info)
  }

  fn store_registry_info(&mut self, info: &RegistryInfo) -> Result<()> {
    store(&self.registry_info, info)
  }

  fn load_publish_info(&self) -> Result<Option<PublishInfo>> {
    load(&self.publish_info)
  }

  fn store_publish_info(&mut self, info: &PublishInfo) -> Result<()> {
    store(&self.publish_info, &info)
  }

  fn clear_publish_info(&mut self) -> Result<()> {
    delete(&self.publish_info)
  }

  fn list_all_packages(&self) -> Result<Vec<String>> {
    let mut packages = Vec::new();
    for entry in self.package_dir()?.read_dir()? {
        let entry = entry?;
        let name = entry.file_name().to_str().unwrap().to_owned();
        packages.push(name);
    }
    Ok(packages)
  }

  fn load_package_state(&self, package: &str) -> Result<package::Validator> {
    load_or_default(&self.package_path(package)?)
  }

  fn store_package_state(
    &mut self,
    package: &str,
    state: &package::Validator,
  ) -> Result<()> {
      store(&self.package_path(package)?, state)
  }

  fn has_content(&self, digest: &DynHash) -> Result<bool> {
    Ok(self.content_path(digest)?.is_file())
  }

  fn store_content<'s>(
    &'s mut self,
    digest: DynHash,
  ) -> Result<Box<dyn ExpectedContent + 's>> {
    let tmp_path = NamedTempFile::new_in(self.temp_dir()?)?.into_temp_path();
    let file = fs::File::create(&tmp_path)?;
    Ok(Box::new(ExpectedFileContent {
        expected_hash: digest,
        storage: self,
        hasher: Sha256::new(),
        tmp_path,
        file,
    }))
  }
    
}