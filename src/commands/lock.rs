use super::CommonOptions;
use anyhow::Result;
use clap::Args;
use std::{fs, path::Path};
use warg_client::{
    storage::{PackageInfo, RegistryStorage},
    FileSystemClient,
};
use warg_crypto::hash::{AnyHash, Sha256};
use warg_protocol::{
    package::ReleaseState,
    registry::{LogId, PackageId, RecordId},
    Version,
};
use wasm_encoder::{
    Component, ComponentExternName, ComponentImportSection,
    ComponentInstanceSection, ComponentTypeRef, ImplementationImport, ImportMetadata,
    ComponentTypeSection,
};
use wasm_lock::Lock;

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
                    Self::lock(&client, &info).await?;
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

    async fn lock(client: &FileSystemClient, info: &PackageInfo) -> Result<()> {
        let mut composed = Component::new();
        let mut content_path =
            String::from("/Users/interpretations/Library/Caches/warg/content/sha256/");
        let imports = ComponentImportSection::new();
        // imports.import(component_extern_name)
        // info.state.releases().for_each(|r| {
        //   if let Some(content) = r.content() {
        //         // let top_meta = ImportMetadata {
        //         //   // name: &info.id.to_string(),

        //         // }
        //         dbg!(&info);
        //         Self::print_release(&r.record_id, &r.version, content);
        //     }
        // });
        let release = info.state.releases().last();
        if let Some(r) = release {
            let state = &r.state;
            if let ReleaseState::Released { content } = state {
                let full_digest = content.to_string();
                let digest = full_digest.split(':').last().unwrap();
                let mut content_path =
                    String::from("/Users/interpretations/Library/Caches/warg/content/sha256/");
                content_path.push_str(&digest);
                let path = Path::new(&content_path);
                let bytes = fs::read(path)?;
                let mut lock = Lock::new();
                let mut component = Component::new();

                let mut name = info.id.to_string();
                name.push_str("/bar");
                dbg!(&name);
                let import = ComponentExternName::Implementation(ImplementationImport::Locked(
                    ImportMetadata {
                        name: &name,
                        location: "",
                        integrity: Some("asldkjf"),
                        range: Some("1.0.0"),
                    },
                ));
                let ty = ComponentTypeRef::Component(0);
                let mut inst_section = ComponentInstanceSection::new();
                let mut imp_section = ComponentImportSection::new();
                let mut type_section = ComponentTypeSection::new();
                imp_section.import(import, ty);
                // component.section(&imp_section);
                let locked = lock.parse(
                    &bytes,
                    &mut component,
                    &mut imp_section,
                    &mut inst_section,
                )?;
                fs::write("./locked.wasm", locked.as_slice())?;
            }
            let version = &r.version.to_string();
            let metadata = ImportMetadata {
                name: &info.id.to_string(),
                location: "",
                integrity: Some("asldkjgd"),
                range: Some(version),
            };
            let extern_name =
                ComponentExternName::Implementation(ImplementationImport::Locked(metadata));
            // imports.import(extern_name);
        }
        let log_id = LogId::package_log::<Sha256>(&info.id);
        let record_id = &info.state.releases().last().unwrap().record_id;
        // let http_client = reqwest::Client::new();
        // let mut req_body = HashMap::new();
        // req_body.insert("logId", log_id.to_string());
        // req_body.insert("recordId", record_id.to_string());
        // let res = http_client
        //     .post("http://127.0.0.1:8090/v1/fetch/dependencies")
        //     .json(&req_body)
        //     .send()
        //     .await?
        //     .json::<FetchDependenciesResponse>()
        //     .await?;
        // let mut tree = TreeBuilder::new(info.id.to_string());
        // for dep in res.dependencies {
        //   let child = tree.begin_child(format!("{0} ({1})", dep.name, dep.version));
        //   let pkg_name = &dep.name.split('/').next().unwrap();
        //   let pkg_id = PackageId::new(pkg_name.to_string())?;
        //   let child = Self::get_dependencies(&http_client, client, &pkg_id, child).await?;
        //   child.end_child();
        // }
        // let built = tree.build();
        // print_tree(&built)?;
        Ok(())
    }

    fn print_release(record_id: &RecordId, version: &Version, content: &AnyHash) {
        println!("    record id: {record_id}");
        println!("    {version} ({content})");
    }
}
