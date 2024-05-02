use super::CommonOptions;
use anyhow::{bail, Result};
use async_recursion::async_recursion;
use clap::Args;
use ptree::{output::print_tree, TreeBuilder};
use std::default::Default;
use std::fs;
use warg_client::{
    storage::PackageInfo,
    version_util::{
        create_child_node, new_tree, version_string, DependencyImportParser, ImportKind,
    },
    FileSystemClient,
};
use warg_protocol::{registry::PackageName, VersionReq};
use wasmparser::{Chunk, ComponentImport, ComponentImportSectionReader, Parser, Payload};

/// Print Dependency Tree
#[derive(Args)]
pub struct DependenciesCommand {
    /// The common command options.
    #[clap(flatten)]
    pub common: CommonOptions,

    /// Only show information for the specified package.
    #[clap(value_name = "PACKAGE")]
    pub package: PackageName,
}

impl DependenciesCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;

        let info = client.package(&self.package).await?;
        Self::print_package_info(&client, &info).await?;

        Ok(())
    }

    #[async_recursion]
    async fn parse_deps<'a>(
        id: &'a PackageName,
        version: VersionReq,
        client: &FileSystemClient,
        node: &mut TreeBuilder,
        parser: &mut DepsParser,
    ) -> Result<()> {
        client.download(id, &version).await?;

        if let Some(download) = client.download(id, &version).await? {
            let bytes = fs::read(download.path)?;
            let deps = parser.parse(&bytes)?;
            for dep in deps {
                let mut dep_parser = DependencyImportParser {
                    next: dep.name.0,
                    offset: 0,
                };
                let dep = dep_parser.parse()?;
                let v = version_string(&dep.req);
                let grand_child = create_child_node(node, &dep.name, &v);
                match dep.kind {
                    ImportKind::Locked(_) | ImportKind::Unlocked => {
                        let id = PackageName::new(dep.name)?;
                        Self::parse_deps(&id, dep.req, client, grand_child, parser).await?;
                    }
                    ImportKind::Interface(_) => {}
                }
                grand_child.end_child();
            }
        }

        Ok(())
    }

    async fn print_package_info(client: &FileSystemClient, info: &PackageInfo) -> Result<()> {
        let mut parser = DepsParser::new();

        if let Some(download) = client.download(&info.name, &Default::default()).await? {
            let mut tree = new_tree(info.name.namespace(), info.name.name(), &download.version);
            let bytes = fs::read(&download.path)?;
            let deps = parser.parse(&bytes)?;
            for dep in deps {
                let mut dep_parser = DependencyImportParser {
                    next: dep.name.0,
                    offset: 0,
                };
                let dep = dep_parser.parse()?;
                let v = version_string(&dep.req);
                let child = create_child_node(&mut tree, &dep.name, &v);
                match dep.kind {
                    ImportKind::Locked(_) | ImportKind::Unlocked => {
                        Self::parse_deps(
                            &PackageName::new(dep.name)?,
                            dep.req,
                            client,
                            child,
                            &mut parser,
                        )
                        .await?;
                    }
                    ImportKind::Interface(_) => {}
                }
                child.end_child();
            }
            let built = tree.build();
            print_tree(&built)?
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
            let (_, imp) = import?;
            deps.push(imp);
        }
        Ok(())
    }

    pub fn parse<'a>(&mut self, mut bytes: &'a [u8]) -> Result<Vec<ComponentImport<'a>>> {
        let mut parser = Parser::new(0);
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
                Payload::CodeSectionStart { .. } => {
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
