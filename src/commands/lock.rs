use super::CommonOptions;
use anyhow::{bail, Result};
use async_recursion::async_recursion;
use clap::Args;
use indexmap::IndexSet;
use semver::{Version, VersionReq};
use std::{collections::HashMap, fs, path::Path};
use warg_client::{
    storage::{PackageInfo, RegistryStorage, ContentStorage},
    FileSystemClient,
};
use warg_protocol::{package::ReleaseState, registry::PackageId};
use wasm_compose::graph::{CompositionGraph, EncodeOptions, ExportIndex, InstanceId};
use wasmparser::{Chunk, ComponentImportSectionReader, Parser, Payload};

/// Builds list of packages in order that they should be added to locked component
pub struct LockListBuilder {
    lock_list: IndexSet<String>,
}

impl LockListBuilder {
    /// New LockListBuilder
    fn new() -> Self {
        Self {
            lock_list: IndexSet::new(),
        }
    }

    fn parse_import(
        &mut self,
        parser: &ComponentImportSectionReader,
        imports: &mut Vec<String>,
    ) -> Result<()> {
        let clone = parser.clone();
        for import in clone.into_iter_with_offsets() {
            let (_, imp) = import.unwrap().clone();
            match imp.name {
                wasmparser::ComponentImportName::Kebab(_) => todo!(),
                wasmparser::ComponentImportName::Interface(name) => {}
                wasmparser::ComponentImportName::Url(metadata) => todo!(),
                wasmparser::ComponentImportName::Relative(metadata) => todo!(),
                wasmparser::ComponentImportName::Naked(metadata) => todo!(),
                wasmparser::ComponentImportName::Locked(metadata) => {}
                wasmparser::ComponentImportName::Unlocked(name) => {
                    imports.push(name.to_string());
                }
            }
        }
        Ok(())
    }

    #[async_recursion]
    async fn parse_package(&mut self, client: &FileSystemClient, mut bytes: &[u8]) -> Result<()> {
        let mut parser = Parser::new(0);
        let mut imports: Vec<String> = Vec::new();
        loop {
            let payload = match parser.parse(bytes, true)? {
                Chunk::NeedMoreData(_) => unreachable!(),
                Chunk::Parsed { payload, consumed } => {
                    bytes = &bytes[consumed..];
                    payload
                }
            };
            match payload {
                Payload::ComponentImportSection(s) => {
                    self.parse_import(&s, &mut imports)?;
                }
                Payload::CodeSectionStart {
                    count: _,
                    range: _,
                    size: _,
                } => {
                    parser.skip_section();
                }
                Payload::ModuleSection { range, .. } => {
                    let offset = range.end - range.start;
                    if offset > bytes.len() {
                        bail!("invalid module or component section range");
                    }
                    bytes = &bytes[offset..];
                }
                Payload::ComponentSection { range, .. } => {
                    let offset = range.end - range.start;
                    if offset > bytes.len() {
                        bail!("invalid module or component section range");
                    }
                    bytes = &bytes[offset..];
                }
                Payload::End(_) => {
                    break;
                }
                _ => {}
            }
        }
        for import in imports {
            let pkg_name = import.split('/').next();
            if let Some(name) = pkg_name {
                let mut name_and_version = name.split('@');
                let identifier = name_and_version.next();
                if let Some(pkg_name) = identifier {
                    let id = PackageId::new(pkg_name)?;
                    if let Some(info) = client.registry().load_package(&id).await? {
                        let release = info.state.releases().last();
                        if let Some(r) = release {
                            let state = &r.state;
                            if let ReleaseState::Released { content } = state {
                                let path = client.content().content_location(content);
                                if let Some(p) = path {
                                  let bytes = fs::read(p)?;
                                  self.parse_package(client, &bytes).await?;
                                }
                            }
                        }
                        self.lock_list.insert(name.to_string());
                    }
                }
            }
        }
        Ok(())
    }

    #[async_recursion]
    async fn build_list(&mut self, client: &FileSystemClient, info: &PackageInfo) -> Result<()> {
        let release = info.state.releases().last();
        if let Some(r) = release {
            let state = &r.state;
            if let ReleaseState::Released { content } = state {
                let path = client.content().content_location(content);
                if let Some(p) = path {
                  let bytes = fs::read(p)?;
                  self.parse_package(client, &bytes).await?;
                }
            }
        }
        Ok(())
    }
}

/// Print Dependency Tree
#[derive(Args)]
pub struct LockCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// Only show information for the specified package.
    #[clap(value_name = "PACKAGE")]
    pub package: Option<PackageId>,

    /// Only show information for the specified package.
    #[clap(long = "exec", value_name = "EXEC", required = false)]
    pub executable: bool,
}

impl LockCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;
        println!("registry: {url}", url = client.url());
        println!("\npackages in client storage:");
        if let Some(package) = self.package {
            if let Some(info) = client.registry().load_package(&package).await? {
                Self::lock(client, &info, self.executable).await?;
            }
        }
        Ok(())
    }

    async fn lock(client: FileSystemClient, info: &PackageInfo, should_bundle: bool) -> Result<()> {
        let maybe_find = |s: &str, c: char| s.find(c);
        let mut builder = LockListBuilder::new();
        builder.build_list(&client, info).await?;
        builder.lock_list.insert(info.id.id.clone());
        let mut composer = CompositionGraph::new();
        let mut handled = HashMap::<String, InstanceId>::new();
        for package in builder.lock_list {
            let mut name_and_version = package.split('@');
            let name = name_and_version.next();
            let version = name_and_version.next();
            if let Some(pkg_id) = name {
                let id = PackageId::new(pkg_id)?;
                let info = client.registry().load_package(&id).await?;
                if let Some(inf) = info {
                    let release = if let Some(v) = version {
                        let mut iterable = v.chars();
                        let (lower, upper) = match iterable.next() {
                            Some('{') => match iterable.next() {
                                Some('>') => {
                                  match iterable.next() {
                                    Some('=') => {
                                      let space = maybe_find(v, ' ');
                                      if let Some(sp) = space {
                                        let lower_bound = &v[3..sp];
                                        let lversion = lower_bound.parse::<Version>()?;
                                        let close = maybe_find(v, '}');
                                        if let Some(c) = close {
                                          let upper_bound = &v[sp + 2..c];
                                          let rversion = upper_bound.parse::<Version>()?;
                                          (Some(lversion), Some(rversion))
                                        } else {
                                          bail!("Range specification missing closing curly brace");
                                        }
                                      } else {
                                        let close = maybe_find(v, '}');
                                        if let Some(c) = close {
                                          let lower_bound = &v[3..c];
                                          let version = lower_bound.parse::<Version>()?;
                                          (Some(version), None)
                                        } else {
                                          bail!("Range specification missing closing curly brace");
                                        }
                                      }
                                    }
                                    _ => bail!("Lower version bound must be inclusive")
                                  }
                                },
                                Some('<') => {
                                  let close = maybe_find(v, '}');
                                  if let Some(c) = close {
                                    let upper_bound = &v[3..c];
                                    let version = upper_bound.parse::<Version>()?;
                                    (None, Some(version))
                                  } else {
                                    bail!("Range specification missing closing curly brace");
                                  } 
                                }
                                _ => {
                                  bail!("Invalid version specification, curly brace usage implies a range should be specified")
                                }
                            },
                            _ => bail!("Invalid version specification, should use curly braces if version is not exact or *"),
                        };
                        match (lower, upper) {
                          (Some(l), Some(u)) => {
                            let req = VersionReq::parse(&format!(">={}, <{}", l, u))?;
                            let matches = inf.state.releases().filter(|r| {
                              req.matches(&r.version)}
                            );
                            matches.last()
                          }
                          (None, Some(u)) => {
                            let req = VersionReq::parse(&format!("<{}", u))?;
                            let matches = inf.state.releases().filter(|r| {
                              req.matches(&r.version)}
                            );
                            matches.last()

                          },
                          (Some(l), None) => {
                            let req = VersionReq::parse(&format!(">={}", l))?;
                            let matches = inf.state.releases().filter(|r| {
                              req.matches(&r.version)}
                            );
                            matches.last()
                          },
                          (None, None) => inf.state.releases().last(),
                        }
                    } else {
                        inf.state.releases().last()
                    };
                    if let Some(r) = release {
                        let state = &r.state;
                        if let ReleaseState::Released { content } = state {
                            let mut locked_package = package.split('@').next().unwrap().to_string();
                            locked_package.push_str(&format!("@{}", &r.version.to_string()));
                            let path = client.content().content_location(content);
                            if let Some(p) = path {
                              let component =
                                  wasm_compose::graph::Component::from_file(locked_package, p)?;
                              let component_index = composer.add_component(component)?;
                              let instance_id = composer.instantiate(component_index)?;
  
                              let added = composer.get_component(component_index);
                              handled.insert(package, instance_id);
                              let mut args = Vec::new();
                              if let Some(added) = added {
                                  for (index, name, _) in added.imports() {
                                      let iid = handled.get(name);
                                      if let Some(arg) = iid {
                                          args.push((arg, index));
                                      }
                                  }
                              }
                              for arg in args {
                                  composer.connect(
                                      *arg.0,
                                      None::<ExportIndex>,
                                      instance_id,
                                      arg.1,
                                  )?;
                              }
                            }
                        }
                    }
                }
            }
        }
        let final_name = &info.id.id;
        let id = handled.get(final_name);
        let options = if let Some(id) = id {
            EncodeOptions {
                define_components: should_bundle,
                export: Some(*id),
                validate: false,
            }
        } else {
            EncodeOptions {
                define_components: false,
                export: None,
                validate: false,
            }
        };
        let locked = composer.encode(options)?;
        fs::write("./locked.wasm", locked.as_slice())?;
        Ok(())
    }
}
