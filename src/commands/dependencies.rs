use super::CommonOptions;
use anyhow::Result;
use async_recursion::async_recursion;
use clap::Args;
use ptree::{item::StringItem, output::print_tree, TreeBuilder};
use reqwest;
use std::collections::HashMap;
use warg_api::v1::fetch::FetchDependenciesResponse;
use warg_client::{
    storage::{PackageInfo, RegistryStorage},
    FileSystemClient,
};
use warg_crypto::hash::{AnyHash, Sha256};
use warg_protocol::{
    registry::{LogId, PackageId, RecordId},
    Version, VersionReq,
};

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
    async fn get_dependency_info<'a>(
        http_client: &reqwest::Client,
        client: &FileSystemClient,
        pkg_id: &PackageId,
        child: &'a mut TreeBuilder,
    ) -> Result<&'a mut TreeBuilder> {
        let pkg = client.registry().load_package(pkg_id).await?;
        if let Some(package) = pkg {
            let record_id = &package.state.releases().last().unwrap().record_id;
            let log_id = LogId::package_log::<Sha256>(pkg_id);
            let mut req_body = HashMap::new();
            req_body.insert("logId", log_id.to_string());
            req_body.insert("recordId", record_id.to_string());
            let res = http_client
                .post("http://127.0.0.1:8090/v1/fetch/dependencies")
                .json(&req_body)
                .send()
                .await?
                .json::<FetchDependenciesResponse>()
                .await?;
            for dep in res.dependencies {
                let pkg_id = PackageId::new(&dep.name)?;
                let grand_child = child.begin_child(format!("{0} ({1})", dep.name, dep.version));
                Self::get_dependency_info(http_client, client, &pkg_id, grand_child).await?;
                grand_child.end_child();
            }
        }
        Ok(child)
    }

    #[async_recursion]
    async fn fetch_deps(client: &FileSystemClient, dep_tree: StringItem) -> Result<()> {
        let mut info = dep_tree.text.split(' ');
        let name = info.next().unwrap();
        // let version = info.next().unwrap();
        let id = PackageId::new(name)?;
        // let stripped = version.replace(['(', ')'], "");
        let version = VersionReq::STAR;
        client.download(&id, &version).await?;
        for child in dep_tree.children {
            Self::fetch_deps(client, child).await?;
        }
        Ok(())
    }

    async fn print_package_info(client: &FileSystemClient, info: &PackageInfo) -> Result<()> {
        info.state.releases().for_each(|r| {
            if let Some(content) = r.content() {
                Self::print_release(&r.record_id, &r.version, content);
            }
        });
        let log_id = LogId::package_log::<Sha256>(&info.id);
        let record_id = &info.state.releases().last().unwrap().record_id;
        let http_client = reqwest::Client::new();
        let mut req_body = HashMap::new();
        req_body.insert("logId", log_id.to_string());
        req_body.insert("recordId", record_id.to_string());
        let res = http_client
            .post("http://127.0.0.1:8090/v1/fetch/dependencies")
            .json(&req_body)
            .send()
            .await?
            .json::<FetchDependenciesResponse>()
            .await?;
        let release = info.state.releases().last();
        if let Some(r) = release {
            let mut tree = TreeBuilder::new(format!("{0} ({1})", info.id, r.version));
            for dep in res.dependencies {
                let child = tree.begin_child(format!("{0} ({1})", dep.name, dep.version));
                let pkg_name = &dep.name.split('/').next().unwrap();
                let pkg_id = PackageId::new(pkg_name.to_string())?;
                let child = Self::get_dependency_info(&http_client, client, &pkg_id, child).await?;
                child.end_child();
            }
            let built = tree.build();
            print_tree(&built)?;
            Self::fetch_deps(&client, built).await?;
        }
        Ok(())
    }

    fn print_release(record_id: &RecordId, version: &Version, content: &AnyHash) {
        println!("    record id: {record_id}");
        println!("    {version} ({content})");
    }
}
