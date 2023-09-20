use super::CommonOptions;
use anyhow::Result;
use clap::Args;
use std::{fs, path::Path};
use warg_client::{
    storage::{FileSystemContentStorage, FileSystemRegistryStorage, RegistryStorage},
    Client,
};
use warg_protocol::{package::ReleaseState, registry::PackageId};
use wasm_encoder::{
    Component, ComponentImportSection, ComponentSectionId, ComponentTypeRef, ImportKind, RawSection,
};
use wasmparser::{Chunk, ComponentImportName, ComponentImportSectionReader, Parser, Payload};

/// Bundle With Registry Dependencies
#[derive(Args)]
pub struct BundleCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// Only show information for the specified package.
    #[clap(value_name = "PACKAGE")]
    pub package: Option<PackageId>,
}

impl BundleCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        println!("registry: {url}", url = client.url());
        println!("\npackages in client storage:");
        let mut bundler = Bundler::new(&client);
        let locked = fs::read("./locked.wasm")?;
        let bundled = bundler.parse(&locked).await?;
        fs::write("./bundled.wasm", bundled.as_slice())?;
        Ok(())
    }
}

/// Bundles Dependencies
pub struct Bundler<'a> {
    client: &'a Client<FileSystemRegistryStorage, FileSystemContentStorage>, // state: State,
                                                                             // max_size: u64,
                                                                             // offset: u64,
                                                                             // encoding: Encoding
}

impl<'a> Bundler<'a> {
    fn new(client: &'a Client<FileSystemRegistryStorage, FileSystemContentStorage>) -> Self {
        Self {
            client, // state: State::Header,
                    // max_size: u64::MAX,
                    // offset,
                    // encoding: Encoding::Module
        }
    }

    async fn parse_imports(
        &mut self,
        parser: ComponentImportSectionReader<'a>,
        component: &mut Component,
    ) -> Result<Vec<u8>> {
        let mut imports = ComponentImportSection::new();
        for import in parser.into_iter_with_offsets() {
            let (_, imp) = import.unwrap().clone();
            match imp.name {
                ComponentImportName::Kebab(name)
                | ComponentImportName::Locked((name, _))
                | ComponentImportName::Unlocked(name) => {
                    let mut full_name = name.split('/');
                    let name = full_name.next();
                    if let Some(name) = name {
                        let mut version_and_name = name.split('@');
                        let identifier = version_and_name.next();
                        let version = version_and_name.next();
                        if let Some(name) = identifier {
                            let pkg_id = PackageId::new(name)?;
                            if let Some(info) = self.client.registry().load_package(&pkg_id).await?
                            {
                                let state = &info.state.releases().last().unwrap().state;
                                if let ReleaseState::Released { content } = state {
                                    let full_digest = content.to_string();
                                    let digest = full_digest.split(':').last().unwrap();
                                    let mut content_path = String::from(
                                      "/Users/interpretations/Library/Caches/warg/content/sha256/",
                                  );
                                    content_path.push_str(&digest);
                                    let path = Path::new(&content_path);
                                    let bytes = fs::read(path)?;
                                    component.section(&RawSection {
                                        id: ComponentSectionId::Component.into(),
                                        data: &bytes,
                                    });
                                }
                            }
                        }
                    }
                }
                ComponentImportName::Interface(name) => match imp.ty {
                    wasmparser::ComponentTypeRef::Module(_) => todo!(),
                    wasmparser::ComponentTypeRef::Func(_) => todo!(),
                    wasmparser::ComponentTypeRef::Value(_) => todo!(),
                    wasmparser::ComponentTypeRef::Type(_) => todo!(),
                    wasmparser::ComponentTypeRef::Instance(i) => {
                        let extern_name = wasm_encoder::ComponentImportName::Interface(&name);
                        imports.import(
                            extern_name,
                            ComponentTypeRef::Instance(i),
                            ImportKind::Interface,
                        );
                    }
                    wasmparser::ComponentTypeRef::Component(_) => todo!(),
                },
                _ => {}
            }
        }
        component.section(&imports);
        Ok(Vec::new())
    }

    async fn parse(&mut self, mut bytes: &'a [u8]) -> Result<Component> {
        let constant = bytes.clone();
        let mut parser = Parser::new(0);
        let mut _consumed = 0;
        let mut component = Component::new();
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
                    self.parse_imports(s, &mut component).await?;
                }
                Payload::ModuleSection { range, .. } => {
                    let offset = range.end - range.start;
                    component.section(&RawSection {
                        id: 1,
                        data: &constant[range],
                    });
                    if offset > bytes.len() {
                        panic!();
                    }
                    bytes = &bytes[offset..];
                }
                Payload::End(_) => {
                    break;
                }
                _ => {
                    if let Some((id, range)) = payload.as_section() {
                        component.section(&RawSection {
                            id,
                            data: &constant[range],
                        });
                    }
                }
            }
        }
        Ok(component)
    }
}
