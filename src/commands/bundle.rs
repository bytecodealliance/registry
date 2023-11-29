use super::{CommonOptions, DependencyImportParser};
use anyhow::Result;
use clap::Args;
use semver::VersionReq;
use std::fs;
use warg_client::{
    storage::{
        ContentStorage, FileSystemContentStorage, FileSystemRegistryStorage, RegistryStorage,
    },
    Client,
};
use warg_protocol::{package::ReleaseState, registry::PackageName};
use wasm_encoder::{
    Component, ComponentImportSection, ComponentSectionId, ComponentTypeRef, RawSection,
};
use wasmparser::{Chunk, ComponentImportSectionReader, Parser, Payload};

/// Bundle With Registry Dependencies
#[derive(Args)]
pub struct BundleCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// Only show information for the specified package.
    #[clap(value_name = "PACKAGE")]
    pub package: Option<PackageName>,
}

impl BundleCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        println!("registry: {url}", url = client.url());
        let mut bundler = Bundler::new(&client);
        let locked = fs::read("./locked.wasm")?;
        let bundled = bundler.parse(&locked).await?;
        fs::write("./bundled.wasm", bundled.as_slice())?;
        Ok(())
    }
}

/// Bundles Dependencies
pub struct Bundler<'a> {
    client: &'a Client<FileSystemRegistryStorage, FileSystemContentStorage>,
}

impl<'a> Bundler<'a> {
    fn new(client: &'a Client<FileSystemRegistryStorage, FileSystemContentStorage>) -> Self {
        Self { client }
    }

    async fn parse_imports(
        &mut self,
        parser: ComponentImportSectionReader<'a>,
        component: &mut Component,
    ) -> Result<Vec<u8>> {
        let mut imports = ComponentImportSection::new();
        for import in parser.into_iter_with_offsets() {
            let (_, imp) = import?;
            let mut dep_parser = DependencyImportParser {
                next: imp.name.0,
                offset: 0,
            };
            let parsed_imp = dep_parser.parse()?;
            if !parsed_imp.name.contains('/') {
                let pkg_id = PackageName::new(parsed_imp.name)?;
                if let Some(info) = self.client.registry().load_package(&pkg_id).await? {
                    let release = if parsed_imp.req != VersionReq::STAR {
                        info.state
                            .releases()
                            .filter(|r| parsed_imp.req.matches(&r.version))
                            .last()
                    } else {
                        info.state.releases().last()
                    };
                    if let Some(r) = release {
                        let release_state = &r.state;
                        if let ReleaseState::Released { content } = release_state {
                            let path = self.client.content().content_location(&content);
                            if let Some(p) = path {
                                let bytes = fs::read(p)?;
                                component.section(&RawSection {
                                    id: ComponentSectionId::Component.into(),
                                    data: &bytes,
                                });
                            }
                        }
                    }
                }
            } else if let wasmparser::ComponentTypeRef::Instance(i) = imp.ty {
                imports.import(imp.name.0, ComponentTypeRef::Instance(i));
            }
        }
        component.section(&imports);
        Ok(Vec::new())
    }

    async fn parse(&mut self, mut bytes: &'a [u8]) -> Result<Component> {
        let constant = bytes.clone();
        let mut parser = Parser::new(0);
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
