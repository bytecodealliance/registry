use async_recursion::async_recursion;
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
    Component, ComponentExternName, ComponentImportSection, ComponentInstanceSection,
    ComponentTypeRef, ComponentTypeSection, ImplementationImport, ImportMetadata, InstanceSection, ComponentExportKind, ComponentExportSection,
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
    #[async_recursion]
    async fn lock_deps<'a>(
        client: &FileSystemClient,
        global_instantiation_args: &'a mut Vec<Vec<String>>,
        component_index: i32,
        packages: &mut Vec<String>,
        component: &'a mut Component,
        imports: &'a mut ComponentImportSection,
        instances: &'a mut ComponentInstanceSection,
    ) -> Result<(&'a mut ComponentImportSection, &'a mut ComponentInstanceSection, &'a mut Component, &'a mut Vec<Vec<String>>)> {
        dbg!(component_index);
        let mut content_path =
            String::from("/Users/interpretations/Library/Caches/warg/content/sha256/");
        let temp_args: Vec<(&str, ComponentExportKind, u32)> = Vec::new();
        for (i, package) in packages.iter().enumerate() {
            let id = PackageId::new(package.to_string())?;
            let info = client.registry().load_package(&id).await?;
            if let Some(inf) = info {

              let release = inf.state.releases().last();
              if let Some(r) = release {
                  let state = &r.state;
                  if let ReleaseState::Released { content } = state {
                      let full_digest = content.to_string();
                      let digest = full_digest.split(':').last().unwrap();
                      content_path.push_str(&digest);
                      let path = Path::new(&content_path);
                      let bytes = fs::read(path)?;
                      // let dep = wasmprinter::print_bytes(&bytes)?;
                      // dbg!(&dep);
                      let mut lock = Lock::new();

                      let mut cur_packages: Vec<String> = Vec::new();
                      let mut nested_packages = lock.parse(
                          &bytes,
                          component,
                          imports,
                          instances,
                          &mut cur_packages,
                      )?.clone();
                      // instances.instantiate(component_index as u32 + i as u32 + 1, temp_args.clone());
                      // dbg!(&cur_packages);
                      // dbg!(nested_packages);
                      global_instantiation_args.push(cur_packages.clone());
                      Self::lock_deps(client, global_instantiation_args, component_index + (i as i32) + 1, &mut cur_packages, component, imports, instances).await?;
                    }
                  }
                }
        }
        // instances.instantiate(component_index as u32, temp_args);
        // component.section(imports);
        Ok((imports, instances, component, global_instantiation_args))
    }

    async fn lock(client: &FileSystemClient, info: &PackageInfo) -> Result<()> {
        let mut content_path =
            String::from("/Users/interpretations/Library/Caches/warg/content/sha256/");
        let mut packages = Vec::new();
        let mut imports = ComponentImportSection::new();
        // let import = ComponentExternName::Implementation(
        //   ImplementationImport::Locked(ImportMetadata {
        //     name: &info.id.id,
        //     location: "",
        //     integrity: Some("asldkjf"),
        //     range: Some("1.0.0")
        //   }));
        //   let ty = ComponentTypeRef::Component(0);

        // imports.import(import, ty);
        let mut component = Component::new();
        let mut inst_section = ComponentInstanceSection::new();
        let release = info.state.releases().last();
        if let Some(r) = release {
            let state = &r.state;
            if let ReleaseState::Released { content } = state {
                let full_digest = content.to_string();
                let digest = full_digest.split(':').last().unwrap();
                content_path.push_str(&digest);
                let path = Path::new(&content_path);
                let bytes = fs::read(path)?;
                let mut lock = Lock::new();

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
                let mut type_section = ComponentTypeSection::new();
                imports.import(import, ty);
                // component.section(&imp_section);
                let locked = lock.parse(
                    &bytes,
                    &mut component,
                    &mut imports,
                    &mut inst_section,
                    &mut packages,
                )?;
            }
        }
        let mut global_instantiation_args = Vec::new();
        let mut instantation_args = Vec::new();
        for package in packages.clone() {
          instantation_args.push(package);
        }
        global_instantiation_args.push(instantation_args);
        let (locked_imports, locked_instances, locked_component, global_instantiation_args) = Self::lock_deps(
            client,
            &mut global_instantiation_args,
            0,
            &mut packages,
            &mut component,
            &mut imports,
            &mut inst_section,
        )
        .await?;
        
        dbg!(&global_instantiation_args);
        global_instantiation_args.reverse();
        let number_of_components = global_instantiation_args.len();
        for (i, comp) in global_instantiation_args.iter().enumerate() {
          let mut temp_args: Vec<(&str, ComponentExportKind, u32)> = Vec::new();
          for (j, arg_name) in comp.iter().enumerate() {
            let arg = ComponentExportKind::Instance {

            };
            temp_args.push((&arg_name, arg, (i + j - 1) as u32));
          }
          locked_instances.instantiate((number_of_components - i - 1) as u32, temp_args);
        }
        locked_component.section(locked_imports);
        locked_component.section(locked_instances);
        let mut exports = ComponentExportSection::new();
        let export = ComponentExternName::Kebab("bundled");
        exports.export(export, ComponentExportKind::Instance, number_of_components as u32 - 1, None);
        component.section(&exports);
        fs::write("./locked.wasm", &component.as_slice())?;
        // let log_id = LogId::package_log::<Sha256>(&info.id);
        // let record_id = &info.state.releases().last().unwrap().record_id;
        Ok(())
    }

    fn print_release(record_id: &RecordId, version: &Version, content: &AnyHash) {
        println!("    record id: {record_id}");
        println!("    {version} ({content})");
    }
}
