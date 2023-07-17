use super::CommonOptions;
use anyhow::Result;
use clap::Args;
use ptree::{output::print_tree, TreeBuilder};
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
    Version,
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
        match self.package {
            Some(package) => {
                if let Some(info) = client.registry().load_package(&package).await? {
                    Self::print_package_info(&client, &info).await?;
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

    async fn print_package_info(client: &FileSystemClient, info: &PackageInfo) -> Result<()> {
        println!("  id: {id}", id = info.id);
        let log_id = LogId::package_log::<Sha256>(&info.id);
        println!("  log id: {log_id}");
        println!("  versions:");
        info.state.releases().for_each(|r| {
            if let Some(content) = r.content() {
                Self::print_release(&r.record_id, &r.version, content);
            }
        });
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
        let mut tree = TreeBuilder::new(info.id.to_string());
        dbg!(&res);
        for dep in res.dependencies {
            let mut node = String::from(&dep.name);
            node.push_str(" (");
            node.push_str(&dep.version);
            node.push(')');
            let child = tree.begin_child(node);
            let pkg_id = PackageId::new(&dep.name)?;
            let pkg = client.registry().load_package(&pkg_id).await?;
            match pkg {
                Some(package) => {
                    let record_id = &package.state.releases().last().unwrap().record_id;
                    let log_id = LogId::package_log::<Sha256>(&pkg_id);
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
                        let mut node = String::from(&dep.name);
                        node.push_str(" (");
                        node.push_str(&dep.version);
                        node.push(')');
                        let grand_child = child.begin_child(node);
                        grand_child.end_child();
                    }
                    child.end_child();
                }
                None => {
                    dbg!("STUFF");
                }
            }

            // .end_child();
        }
        let built = tree.build();
        print_tree(&built)?;
        Ok(())
    }

    fn print_release(record_id: &RecordId, version: &Version, content: &AnyHash) {
        println!("    record id: {record_id}");
        println!("    {version} ({content})");
    }
}
