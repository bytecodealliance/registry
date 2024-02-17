use super::{CommonOptions, Retry};
use anyhow::{bail, Result};
use async_recursion::async_recursion;
use clap::Args;
use ptree::{output::print_tree, TreeBuilder};
use std::fs;
use warg_client::{
    storage::{ContentStorage, PackageInfo, RegistryStorage},
    version_util::{
        create_child_node, new_tree, version_string, DependencyImportParser, ImportKind,
    },
    FileSystemClient,
};
use warg_protocol::{package::ReleaseState, registry::PackageName, VersionReq};
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
    pub async fn exec(self, retry: Option<Retry>) -> Result<()> {
        let config = self.common.read_config()?;
        let mut client = self.common.create_client(&config)?;
        let auth_token = self.common.auth_token()?;
        if let Some(retry) = retry {
            retry.store_namespace(&client).await?
        }
        client
            .refresh_namespace(&auth_token, self.package.namespace())
            .await?;

        if let Some(info) = client
            .registry()
            .load_package(client.get_warg_header(), &self.package)
            .await?
        {
            Self::print_package_info(&auth_token, &client, &info).await?;
        }

        Ok(())
    }

    #[async_recursion]
    async fn parse_deps<'a>(
        auth_token: &Option<String>,
        id: &'a PackageName,
        version: VersionReq,
        client: &FileSystemClient,
        node: &mut TreeBuilder,
        parser: &mut DepsParser,
    ) -> Result<()> {
        client.download(auth_token, id, &version).await?;

        let package = client
            .registry()
            .load_package(client.get_warg_header(), id)
            .await?;
        if let Some(pkg) = package {
            let latest = pkg.state.releases().last();
            if let Some(l) = latest {
                if let ReleaseState::Released { content } = &l.state {
                    let path = client.content().content_location(content);
                    if let Some(p) = path {
                        let bytes = fs::read(p)?;
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
                                    Self::parse_deps(
                                        auth_token,
                                        &id,
                                        dep.req,
                                        client,
                                        grand_child,
                                        parser,
                                    )
                                    .await?;
                                }
                                ImportKind::Interface(_) => {}
                            }
                            grand_child.end_child();
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn print_package_info(
        auth_token: &Option<String>,
        client: &FileSystemClient,
        info: &PackageInfo,
    ) -> Result<()> {
        let mut parser = DepsParser::new();
        let root_package = client
            .registry()
            .load_package(client.get_warg_header(), &info.name)
            .await?;
        if let Some(rp) = root_package {
            let latest = rp.state.releases().last();
            if let Some(l) = latest {
                client
                    .download(auth_token, &info.name, &VersionReq::STAR)
                    .await?;
                let mut tree = new_tree(info.name.namespace(), info.name.name(), &l.version);
                if let ReleaseState::Released { content } = &l.state {
                    let path = client.content().content_location(content);
                    if let Some(p) = path {
                        let bytes = fs::read(&p)?;
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
                                        auth_token,
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
                    }
                    let built = tree.build();
                    print_tree(&built)?
                }
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
