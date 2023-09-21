use super::CommonOptions;
use anyhow::{bail, Result};
use async_recursion::async_recursion;
use clap::Args;
use ptree::{output::print_tree, TreeBuilder};
use std::fs;
use warg_client::{
    storage::{ContentStorage, PackageInfo, RegistryStorage},
    FileSystemClient,
};
use warg_protocol::{package::ReleaseState, registry::PackageId, VersionReq};
use wasmparser::ComponentImportName;
use wasmparser::{Chunk, ComponentImport, ComponentImportSectionReader, Parser, Payload};

/// Print Dependency Tree
#[derive(Args)]
pub struct DependenciesCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// Only show information for the specified package.
    #[clap(value_name = "PACKAGE")]
    pub package: Option<PackageId>,
}

impl DependenciesCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        if let Some(package) = self.package {
            if let Some(info) = client.registry().load_package(&package).await? {
                Self::print_package_info(&client, &info).await?;
            }
        }

        Ok(())
    }

    #[async_recursion]
    async fn parse_deps<'a>(
        id: &'a PackageId,
        version: Option<&'a str>,
        client: &FileSystemClient,
        node: &mut TreeBuilder,
        parser: &mut DepsParser,
    ) -> Result<()> {
        let vreq = if let Some(v) = version {
            let v = v.replace(['{', '}'], "").replace([' '], ", ");
            VersionReq::parse(&v)
        } else {
            Ok(VersionReq::STAR)
        }?;
        client.download(id, &vreq).await?;

        let package = client.registry().load_package(id).await?;
        if let Some(pkg) = package {
            let latest = pkg.state.releases().last();
            if let Some(l) = latest {
                if let ReleaseState::Released { content } = &l.state {
                    let path = client.content().content_location(content);
                    if let Some(p) = path {
                        let bytes = fs::read(p)?;
                        let deps = parser.parse(&bytes)?;
                        for dep in deps {
                            if let ComponentImportName::Unlocked(name) = dep.name {
                                let mut name_and_version = name.split('@');
                                let versionless_name = name_and_version.next();
                                let version = name_and_version.next();
                                if let Some(identifier) = versionless_name {
                                    let grand_child = node.begin_child(name.to_string());
                                    let id = PackageId::new(identifier)?;
                                    Self::parse_deps(&id, version, client, grand_child, parser)
                                        .await?;
                                    grand_child.end_child();
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn print_package_info(client: &FileSystemClient, info: &PackageInfo) -> Result<()> {
        let mut parser = DepsParser::new();
        let root_package = client.registry().load_package(&info.id).await?;
        if let Some(rp) = root_package {
            let latest = rp.state.releases().last();
            if let Some(l) = latest {
                let mut tree = TreeBuilder::new(format!("{0}@{1}", info.id, l.version));
                if let ReleaseState::Released { content } = &l.state {
                    let path = client.content().content_location(content);
                    if let Some(p) = path {
                        let bytes = fs::read(&p)?;
                        let deps = parser.parse(&bytes)?;
                        for dep in deps {
                            if let ComponentImportName::Unlocked(name) = dep.name {
                                let child = tree.begin_child(name.to_string());
                                let mut name_and_version = name.split('@');
                                let versionless_name = name_and_version.next();
                                let version = name_and_version.next();
                                if let Some(identifier) = versionless_name {
                                    let id = PackageId::new(identifier)?;
                                    Self::parse_deps(&id, version, client, child, &mut parser)
                                        .await?;
                                }
                                child.end_child();
                            }
                        }
                    }
                }
                let built = tree.build();
                print_tree(&built)?
            }
        }
        Ok(())
    }
}

struct DepsParser {}

impl DepsParser {
    pub fn new() -> Self {
        Self {}
    }

    pub fn parse_imports<'a>(
        &mut self,
        parser: ComponentImportSectionReader<'a>,
        deps: &mut Vec<ComponentImport<'a>>,
    ) -> Result<()> {
        for import in parser.into_iter_with_offsets() {
            let (_, imp) = import.unwrap();
            deps.push(imp);
        }
        Ok(())
    }

    pub fn parse<'a>(&mut self, mut bytes: &'a [u8]) -> Result<Vec<ComponentImport<'a>>> {
        let mut parser = Parser::new(0);
        let mut _consumed = 0;
        let mut deps = Vec::new();
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
                    self.parse_imports(s, &mut deps)?;
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
        Ok(deps)
    }
}
