use super::CommonOptions;
use anyhow::{bail, Result};
use async_recursion::async_recursion;
use clap::Args;
use indexmap::IndexSet;
use std::{collections::HashSet, fs, path::Path};
use warg_client::{
    storage::{PackageInfo, RegistryStorage},
    FileSystemClient,
};
use warg_protocol::{package::ReleaseState, registry::PackageId};
use wasm_encoder::{
    Component, ComponentExportKind, ComponentExportSection, ComponentExternName,
    ComponentImportSection, ComponentInstanceSection, ComponentTypeRef, ImplementationImport,
};
use wasm_lock::{Dependency, Graph, Import, ImportKind, Lock};
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
        // let mut imports = Vec::new();
        for (i, import) in clone.into_iter_with_offsets().enumerate() {
            let (_, imp) = import.unwrap().clone();
            match imp.name {
                wasmparser::ComponentExternName::Kebab(_) => todo!(),
                wasmparser::ComponentExternName::Interface(name) => {
                    // self.lock_list.insert(name.to_string(),);
                }
                wasmparser::ComponentExternName::Implementation(imp) => match imp {
                    wasmparser::ImplementationImport::Url(metadata) => todo!(),
                    wasmparser::ImplementationImport::Relative(metadata) => todo!(),
                    wasmparser::ImplementationImport::Naked(metadata) => todo!(),
                    wasmparser::ImplementationImport::Locked(metadata) => {

                    },
                    wasmparser::ImplementationImport::Unlocked(metadata) => {
                        imports.push(metadata.name.to_string());
                    }
                },
            }
        }
        Ok(())
    }

    #[async_recursion]
    async fn parse_package(
        &mut self,
        client: &FileSystemClient,
        mut bytes: &[u8],
        mut graph: &mut Graph,
    ) -> Result<()> {
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
                            self.parse_package(client, &bytes, graph).await?;
                        }
                    }
                    self.lock_list.insert(info.id.id.to_string());
                }
            }
        }
        Ok(())
    }

    #[async_recursion]
    async fn build_list(
        &mut self,
        client: &FileSystemClient,
        info: &PackageInfo,
        mut graph: &mut Graph,
    ) -> Result<()> {
        let mut content_path =
            String::from("/Users/interpretations/Library/Caches/warg/content/sha256/");
        let release = info.state.releases().last();
        let name = info.id.id.clone();
        if let Some(r) = release {
            let state = &r.state;
            if let ReleaseState::Released { content } = state {
                let full_digest = content.to_string();
                let digest = full_digest.split(':').last().unwrap();
                content_path.push_str(digest);
                let path = Path::new(&content_path);
                let bytes = fs::read(path)?;
                self.parse_package(client, &bytes, graph).await?;
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
}

impl LockCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        println!("registry: {url}", url = client.url());
        println!("\npackages in client storage:");
        match self.package {
            Some(package) => {
                if let Some(info) = client.registry().load_package(&package).await? {
                    Self::lock(client, &info).await?;
                }
            }
            None => {
                // client
                //     .registry()
                //     .load_packages()
                //     .await?
                //     .iter()
                //     .for_each(Self::print_package_info);
            }
        }

        Ok(())
    }

    async fn lock(client: FileSystemClient, info: &PackageInfo) -> Result<()> {
        let mut graph = Graph::new();
        let mut builder = LockListBuilder::new();
        builder.build_list(&client, info, &mut graph).await?;
        builder.lock_list.insert(info.id.id.clone());
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
                        let bytes = fs::read(path)?;
                        let mut projected = String::new();
                        projected.push_str(&inf.id.id);
                        projected.push_str("/bar");
                        graph.insert_component(
                            Import::new(inf.id.id, ImportKind::Implementation),
                            Dependency::new(graph.num_components),
                        );
                        let mut lock = Lock::new();
                        lock.parse_new(&bytes, &mut graph)?;
                    }
                }
            }
        }
        let mut locked_component = Component::new();
        let mut index = 0;
        let mut imported = HashSet::<String>::new();
        for entity in graph.entities.clone() {
            match entity {
                wasm_lock::Entity::Type(ty) => {
                    locked_component.section(&ty);
                }
                wasm_lock::Entity::Alias(ty) => {
                    locked_component.section(&ty);
                }
                wasm_lock::Entity::Import((import, ty)) => match import.kind {
                    ImportKind::Implementation => {
                        let dep = graph.components.get(&import.name);
                        if let Some(imp) = dep {
                            if !imported.contains(&import.name) {
                                let mut locked_instance = ComponentInstanceSection::new();
                                locked_component.section(&ty);
                                let mut temp_args: Vec<(&str, ComponentExportKind, u32)> =
                                    Vec::new();
                                for (_, arg) in imp.instantiation_args.iter().enumerate() {
                                    let exp = ComponentExportKind::Instance;
                                    let arg_dep = graph.components.get(arg);
                                    if let Some(ad) = arg_dep {
                                        if let Some(instance_index) = ad.instance {
                                            temp_args.push((arg, exp, instance_index as u32));
                                        }
                                    }
                                    let instance_arg_dep = graph.interfaces.get(arg);
                                    if let Some(interface) = instance_arg_dep {
                                        temp_args.push((arg, exp, (*interface - 1) as u32));
                                    }
                                }
                                locked_instance.instantiate(index as u32, temp_args);
                                graph.insert_instance(import.name.clone(), graph.num_instances);
                                locked_component.section(&locked_instance);
                                imported.insert(import.name);
                                index += 1;
                            }
                        }
                    }
                    ImportKind::Interface => {
                        locked_component.section(&ty);
                    }
                    ImportKind::Kebab => {},
                },
            }
        }

        let mut top_level = ComponentImportSection::new();
        let top_name = &info.id.id;
        top_level.import(
            ComponentExternName::Implementation(ImplementationImport::Locked(
                wasm_encoder::ImportMetadata {
                    name: &top_name,
                    location: "",
                    integrity: Some(""),
                },
            )),
            ComponentTypeRef::Component(0),
        );
        locked_component.section(&top_level);
        let dep = graph.components.get(top_name);
        if let Some(dep) = dep {
            let mut locked_instance = ComponentInstanceSection::new();
            let mut temp_args: Vec<(&str, ComponentExportKind, u32)> = Vec::new();
            for (_, arg) in dep.instantiation_args.iter().enumerate() {
                let exp = ComponentExportKind::Instance;
                let arg_dep = graph.components.get(arg);
                if let Some(ad) = arg_dep {
                    if let Some(instance_index) = ad.instance {
                        temp_args.push((arg, exp, instance_index as u32));
                    }
                }
            }

            locked_instance.instantiate(index as u32, temp_args);
            graph.insert_instance(top_name.to_string(), graph.num_instances);
            locked_component.section(&locked_instance);
        }

        let mut exports = ComponentExportSection::new();
        let export = ComponentExternName::Kebab("bundled");
        exports.export(
            export,
            ComponentExportKind::Instance,
            (graph.num_components + graph.num_interfaces) as u32 - 1,
            None,
        );
        locked_component.section(&exports);
        fs::write("./locked.wasm", locked_component.as_slice())?;
        Ok(())
    }
}
