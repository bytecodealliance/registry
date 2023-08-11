use super::CommonOptions;
use anyhow::Result;
use async_recursion::async_recursion;
use clap::Args;
use std::{collections::HashMap, fs, path::Path};
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
    Component, ComponentExportKind, ComponentExportSection, ComponentExternName,
    ComponentImportSection, ComponentInstanceSection, ComponentTypeRef, ComponentTypeSection,
    ImplementationImport, ImportMetadata, InstanceSection,
};
use wasm_lock::Lock;

struct Graph {
    num_components: usize,
    num_instances: usize,
    dependencies: HashMap<String, Dependency>,
    indices: HashMap<usize, String>,
}

impl Graph {
    fn new() -> Self {
        Self {
            num_components: 0,
            num_instances: 0,
            dependencies: HashMap::new(),
            indices: HashMap::new(),
        }
    }

    fn insert_component(&mut self, key: String, val: Dependency) {
        if let std::collections::hash_map::Entry::Vacant(e) = self.dependencies.entry(key.clone()) {
            self.indices.insert(self.num_components, key);
            self.num_components += 1;
            e.insert(val);
        }
    }

    fn insert_instance(&mut self, key: String, val: usize) {
        self.num_instances += 1;
        self.dependencies.get_mut(&key).map(|comp| {
            comp.instance = Some(val);
        });
    }
}

#[derive(Debug)]
struct Dependency {
    index: usize,
    instance: Option<usize>,
    instantiation_args: Vec<String>,
}

impl Dependency {
    fn new(index: usize) -> Self {
        Self {
            index,
            instance: None,
            instantiation_args: Vec::new(),
        }
    }
}
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
        global_components: &mut Graph,
        // global_instantiation_args: &'a mut Vec<Vec<String>>,
        component_index: i32,
        packages: &mut Vec<String>,
        component: &'a mut Component,
        imports: &'a mut ComponentImportSection,
        instances: &'a mut ComponentInstanceSection,
    ) -> Result<(
        &'a mut ComponentImportSection,
        &'a mut ComponentInstanceSection,
        &'a mut Component,
        // &'a mut Vec<Vec<String>>,
    )> {
        dbg!(component_index);
        for (_, package) in packages.iter().enumerate() {
            // let mut projected = String::new();
            // projected.push_str(package);
            // projected.push_str("/bar");
            // dbg!(&projected);
            global_components.insert_component(
                package.clone(),
                Dependency::new(global_components.num_components + 1),
            );
            // dep.instantiation_args.push(package.to_string());
        }
        let root = global_components.indices.get(&(component_index as usize));

        // let temp_args: Vec<(&str, ComponentExportKind, u32)> = Vec::new();

        for (i, package) in packages.iter().enumerate() {
            dbg!(&package);
            let name = package.split('/').next();
            if let Some(id) = name {
              let mut content_path =
                  String::from("/Users/interpretations/Library/Caches/warg/content/sha256/");
              let id = PackageId::new(id.to_string())?;
              dbg!(&id);
              let info = client.registry().load_package(&id).await?;
              dbg!("LOADED");
            if let Some(inf) = info {
                let release = inf.state.releases().last();
                if let Some(r) = release {
                    let state = &r.state;
                    if let ReleaseState::Released { content } = state {
                        let full_digest = content.to_string();
                        let digest = full_digest.split(':').last().unwrap();
                        content_path.push_str(&digest);
                        let path = Path::new(&content_path);
                        dbg!(&path);
                        let bytes = fs::read(path)?;
                        let mut lock = Lock::new();

                        let mut cur_packages: Vec<String> = Vec::new();
                        let mut nested_packages = lock
                            .parse(&bytes, component, imports, instances, &mut cur_packages)?
                            .clone();
                        // let mut projected = String::new();
                        // projected.push_str(package);
                        // projected.push_str("/bar");
                        // dbg!(&projected);
                        let comp = global_components.dependencies.get_mut(package);
                        if let Some(c) = comp {
                            for pkg in cur_packages.clone() {
                                let mut projected = String::new();
                                // projected.push_str(&pkg);
                                // projected.push_str("/bar");
                                c.instantiation_args.push(pkg);
                            }
                        }
                        // global_instantiation_args.push(cur_packages.clone());
                        Self::lock_deps(
                            client,
                            global_components,
                            // global_instantiation_args,
                            component_index + (i as i32) + 1,
                            &mut cur_packages,
                            component,
                            imports,
                            instances,
                        )
                        .await?;
                    }
                }
            }
          }
        }
        // instances.instantiate(component_index as u32, temp_args);
        // component.section(imports);
        Ok((
            imports, instances, component,
            // global_instantiation_args
        ))
    }

    async fn lock(client: &FileSystemClient, info: &PackageInfo) -> Result<()> {
        let mut content_path =
            String::from("/Users/interpretations/Library/Caches/warg/content/sha256/");
        let mut packages = Vec::new();
        let mut imports = ComponentImportSection::new();
        let mut global_components = Graph::new();
        let mut projected = String::new();
        projected.push_str(&info.id.id);
        projected.push_str("/bar");
        global_components.insert_component(projected, Dependency::new(0));
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
                let locked = lock.parse(
                    &bytes,
                    &mut component,
                    &mut imports,
                    &mut inst_section,
                    &mut packages,
                )?;
            }
        }
        // let mut global_instantiation_args = Vec::new();
        // global_instantiation_args.push(instantation_args);
        // global_components.push
        dbg!(&packages);
        let mut projected = String::new();
        projected.push_str(&info.id.id);
        projected.push_str("/bar");
        let comp = global_components.dependencies.get_mut(&projected);
        if let Some(dep) = comp {
            for package in packages.clone() {
                // let mut projected = String::new();
                // projected.push_str(&package);
                // projected.push_str("/bar");
                // dbg!(&projected);
                dep.instantiation_args.push(package);
            }
            dbg!(&dep);
        }
        let (
            locked_imports,
            locked_instances,
            locked_component,
            //  global_instantiation_args
        ) = Self::lock_deps(
            client,
            &mut global_components,
            // &mut global_instantiation_args,
            0,
            &mut packages,
            &mut component,
            &mut imports,
            &mut inst_section,
        )
        .await?;

        // dbg!(&global_instantiation_args);
        // global_instantiation_args.reverse();
        for i in 0..global_components.num_components {
            dbg!(i);
            let comp = global_components.indices.get(&i);
            if let Some(name) = comp {
                // let mut projected = String::new();
                // projected.push_str(name);
                // projected.push_str("/bar");
                // dbg!(&projected);

                let import = ComponentExternName::Implementation(ImplementationImport::Locked(
                    ImportMetadata {
                        name,
                        location: "",
                        integrity: Some("asldkjf"),
                        range: Some("1.0.0"),
                    },
                ));

                let ty = ComponentTypeRef::Component(0);
                locked_imports.import(import, ty);
            }
        }
        dbg!(&global_components.indices);
        dbg!(&global_components.dependencies);
        for i in 0..global_components.num_components {
            let mut temp_args: Vec<(&str, ComponentExportKind, u32)> = Vec::new();
            let indices = global_components.indices.clone();
            let pkg_name = indices.get(&(global_components.num_components - i - 1));
            if let Some(name) = pkg_name {
                let pkg = global_components.dependencies.get_mut(&name.clone());
                if let Some(dep) = pkg {
                    dep.instance = Some(i);
                }
                let read_pkg = global_components.dependencies.get(&name.clone());
                if let Some(dep) = read_pkg {
                    for (_, arg) in dep.instantiation_args.iter().enumerate() {
                        let exp = ComponentExportKind::Instance;
                        let arg_dep = global_components.dependencies.get(arg);
                        dbg!(&arg);
                        if let Some(ad) = arg_dep {
                            if let Some(instance_index) = ad.instance {
                                temp_args.push((arg, exp, instance_index as u32))
                            }
                        }
                    }
                    dbg!(&temp_args);
                    // dbg!(&dep);
                    locked_instances
                        .instantiate((global_components.num_components - i - 1) as u32, temp_args);
                }
            }
        }
        // dbg!(global_components.get())
        // }
        locked_component.section(locked_imports);
        locked_component.section(locked_instances);
        let mut exports = ComponentExportSection::new();
        let export = ComponentExternName::Kebab("bundled");
        exports.export(
            export,
            ComponentExportKind::Instance,
            global_components.num_components as u32 - 1,
            None,
        );
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
