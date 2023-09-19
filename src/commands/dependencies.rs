use super::CommonOptions;
use anyhow::Result;
use async_recursion::async_recursion;
use clap::Args;
use ptree::{output::print_tree, TreeBuilder};
use std::fs;
use warg_client::{
    storage::{PackageInfo, RegistryStorage},
    FileSystemClient,
};
use warg_crypto::hash::AnyHash;
use warg_protocol::{
    package::ReleaseState,
    registry::{PackageId, RecordId},
    Version, VersionReq,
};
use wasm_deps::{self, DepsParser};
use wasmparser::ComponentImportName;

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

        println!("registry: {url}", url = client.url());
        println!("\npackages in client storage:");
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
            dbg!(&v);
            let v = v.replace(['{', '}'], "");
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
                    let stringified = content.to_string();
                    let sha = stringified.split(':').last();
                    if let Some(sha) = sha {
                        let path = format!(
                            "/Users/interpretations/Library/Caches/warg/content/sha256/{}",
                            sha
                        );
                        let bytes = fs::read(path)?;
                        let deps = parser.parse(&bytes)?;
                        dbg!(&deps);
                        for dep in deps {
                            dbg!(&dep.name);
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
                    let stringified = content.to_string();
                    let sha = stringified.split(':').last();
                    if let Some(sha) = sha {
                        let path = format!(
                            "/Users/interpretations/Library/Caches/warg/content/sha256/{}",
                            sha
                        );
                        let bytes = fs::read(&path)?;
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

    fn print_release(record_id: &RecordId, version: &Version, content: &AnyHash) {
        println!("    record id: {record_id}");
        println!("    {version} ({content})");
    }
}
