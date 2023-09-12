use super::CommonOptions;
use anyhow::{bail, Result};
use async_recursion::async_recursion;
use clap::Args;
use indexmap::IndexSet;
use std::{collections::HashMap, fs, path::Path};
use warg_client::{
    storage::{PackageInfo, RegistryStorage},
    FileSystemClient,
};
use warg_protocol::{package::ReleaseState, registry::PackageId};
use wasm_compose::graph::{CompositionGraph, EncodeOptions, ExportIndex, InstanceId};
use wasmparser::{Chunk, ComponentImportName, ComponentImportSectionReader, Parser, Payload};

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
                Payload::ModuleSection { parser, range } => {
                    let offset = range.end - range.start;
                    if offset > bytes.len() {
                        bail!("invalid module or component section range");
                    }
                    bytes = &bytes[offset..];
                }
                Payload::ComponentSection { parser, range } => {
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
                let id = PackageId::new(name)?;
                if let Some(info) = client.registry().load_package(&id).await? {
                    let mut content_path =
                        String::from("/Users/interpretations/Library/Caches/warg/content/sha256/");
                    let release = info.state.releases().last();
                    if let Some(r) = release {
                        let state = &r.state;
                        if let ReleaseState::Released { content } = state {
                            let full_digest = content.to_string();
                            let digest = full_digest.split(':').last().unwrap();
                            content_path.push_str(digest);
                            let path = Path::new(&content_path);
                            let bytes = fs::read(path)?;
                            self.parse_package(client, &bytes).await?;
                        }
                    }
                    self.lock_list.insert(id.to_string());
                }
            }
        }
        Ok(())
    }

    #[async_recursion]
    async fn build_list(&mut self, client: &FileSystemClient, info: &PackageInfo) -> Result<()> {
        let mut content_path =
            String::from("/Users/interpretations/Library/Caches/warg/content/sha256/");
        let release = info.state.releases().last();
        if let Some(r) = release {
            let state = &r.state;
            if let ReleaseState::Released { content } = state {
                let full_digest = content.to_string();
                let digest = full_digest.split(':').last().unwrap();
                content_path.push_str(digest);
                let path = Path::new(&content_path);
                let bytes = fs::read(path)?;
                self.parse_package(client, &bytes).await?;
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
        let mut builder = LockListBuilder::new();
        builder.build_list(&client, info).await?;
        builder.lock_list.insert(info.id.id.clone());
        let mut composer = CompositionGraph::new();
        let mut handled = HashMap::<String, InstanceId>::new();
        for package in builder.lock_list {
            let id = PackageId::new(package)?;
            let info = client.registry().load_package(&id).await?;
            if let Some(inf) = info {
                let release = inf.state.releases().last();
                if let Some(r) = release {
                    let state = &r.state;
                    if let ReleaseState::Released { content } = state {
                        let full_digest = content.to_string();
                        let digest = full_digest.split(':').last().unwrap();
                        let mut content_path = String::from(
                            "/Users/interpretations/Library/Caches/warg/content/sha256/",
                        );
                        content_path.push_str(digest);
                        let path = Path::new(&content_path);
                        let component =
                            wasm_compose::graph::Component::from_file(inf.id.id.clone(), path)?;
                        let component_index = composer.add_component(component)?;
                        let instance_id = composer.instantiate(component_index)?;
                        let added = composer.get_component(component_index);
                        let name = inf.id.id.clone();
                        handled.insert(name, instance_id);
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
                            composer.connect(*arg.0, None::<ExportIndex>, instance_id, arg.1)?;
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
                define_components: true,
                export: None,
                validate: false,
            }
        };
        let locked = composer.encode(options)?;
        fs::write("./locked.wasm", locked.as_slice())?;
        Ok(())
    }
}
