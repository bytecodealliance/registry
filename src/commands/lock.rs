use super::CommonOptions;
use anyhow::{bail, Result};
use async_recursion::async_recursion;
use clap::Args;
use indexmap::IndexSet;
use semver::{Comparator, Prerelease, Version, VersionReq};
use std::{collections::HashMap, fs};
use warg_client::{
    storage::{ContentStorage, PackageInfo, RegistryStorage},
    FileSystemClient,
};
use warg_protocol::{package::ReleaseState, registry::PackageName};
use wasm_compose::graph::{CompositionGraph, EncodeOptions, ExportIndex, InstanceId};
use wasmparser::{names::KebabStr, Chunk, ComponentImportSectionReader, Parser, Payload};

struct ResolutionParser<'a> {
    next: &'a str,
    offset: usize,
}

#[derive(Debug, Eq, PartialEq, Hash)]
struct Import {
    name: String,
    req: VersionReq,
}

impl<'a> ResolutionParser<'a> {
    fn parse(&mut self) -> Result<Import> {
        dbg!(&self.next);
        if self.eat_str("unlocked-dep=") {
            self.expect_str("<")?;
            let imp = self.pkgidset_up_to('>')?;
            self.expect_str(">")?;
            return Ok(imp);
        }
        bail!("expected unlocked dep");
    }

    fn eat_str(&mut self, prefix: &str) -> bool {
        match self.next.strip_prefix(prefix) {
            Some(rest) => {
                self.next = rest;
                true
            }
            None => false,
        }
    }

    fn expect_str(&mut self, prefix: &str) -> Result<()> {
        if self.eat_str(prefix) {
            Ok(())
        } else {
            bail!(format!(
                "expected `{prefix}` at `{}` at {}",
                self.next, self.offset
            ));
        }
    }

    fn eat_up_to(&mut self, c: char) -> Option<&'a str> {
        let i = self.next.find(c)?;
        let (a, b) = self.next.split_at(i);
        self.next = b;
        Some(a)
    }

    fn eat_until(&mut self, c: char) -> Option<&'a str> {
        let ret = self.eat_up_to(c);
        if ret.is_some() {
            self.next = &self.next[c.len_utf8()..];
        }
        ret
    }

    fn kebab(&self, s: &'a str) -> Result<&'a KebabStr> {
        match KebabStr::new(s) {
            Some(name) => Ok(name),
            None => bail!(format!("`{s}` is not in kebab case at {}", self.offset)),
        }
    }

    fn take_until(&mut self, c: char) -> Result<&'a str> {
        match self.eat_until(c) {
            Some(s) => Ok(s),
            None => bail!(format!("failed to find `{c}` character at {}", self.offset)),
        }
    }

    fn take_up_to(&mut self, c: char) -> Result<&'a str> {
        match self.eat_up_to(c) {
            Some(s) => Ok(s),
            None => bail!(format!("failed to find `{c}` character at {}", self.offset)),
        }
    }

    fn semver(&self, s: &str) -> Result<Version> {
        match Version::parse(s) {
            Ok(v) => Ok(v),
            Err(e) => bail!(format!(
                "`{s}` is not a valid semver: {e} at {}",
                self.offset
            )),
        }
    }

    fn pkgidset_up_to(&mut self, end: char) -> Result<Import> {
        let namespace = self.take_until(':')?;
        self.kebab(namespace)?;
        let name = match self.eat_until('@') {
            Some(name) => name,
            // a:b
            None => {
                let name = self.take_up_to(end)?;
                self.kebab(name)?;
                return Ok(Import {
                    name: format!("{namespace}:{name}"),
                    req: VersionReq::STAR,
                });
            }
        };
        self.kebab(name)?;

        // a:b@*
        if self.eat_str("*") {
            return Ok(Import {
                name: format!("{namespace}:{name}"),
                req: VersionReq::STAR,
            });
        }
        self.expect_str("{")?;
        if self.eat_str(">=") {
            match self.eat_until(' ') {
                Some(lower) => {
                    let lower = self.semver(lower)?;
                    self.expect_str(">")?;
                    let upper = self.take_until('}')?;
                    let upper = self.semver(upper)?;
                    let lc = Comparator {
                        op: semver::Op::GreaterEq,
                        major: lower.major,
                        minor: Some(lower.minor),
                        patch: Some(lower.patch),
                        pre: Prerelease::default(),
                    };
                    let uc = Comparator {
                        op: semver::Op::Less,
                        major: upper.major,
                        minor: Some(upper.minor),
                        patch: Some(upper.patch),
                        pre: Prerelease::default(),
                    };
                    let mut comparators = Vec::new();
                    comparators.push(lc);
                    comparators.push(uc);
                    return Ok(Import {
                        name: format!("{namespace}:{name}"),
                        req: VersionReq { comparators },
                    });
                }
                // a:b@{>=1.2.3}
                None => {
                    let lower = self.take_until('}')?;
                    let lower = self.semver(lower)?;
                    let comparator = Comparator {
                        op: semver::Op::GreaterEq,
                        major: lower.major,
                        minor: Some(lower.minor),
                        patch: Some(lower.patch),
                        pre: Prerelease::default(),
                    };
                    let mut comparators = Vec::new();
                    comparators.push(comparator);
                    return Ok(Import {
                        name: format!("{namespace}:{name}"),
                        req: VersionReq { comparators },
                    });
                }
            }
        }

        // a:b@{<1.2.3}
        // .. or
        // a:b@{<1.2.3 >=1.2.3}
        self.expect_str("<")?;
        let upper = self.take_until('}')?;
        let upper = self.semver(upper)?;
        let uc = Comparator {
            op: semver::Op::Less,
            major: upper.major,
            minor: Some(upper.minor),
            patch: Some(upper.patch),
            pre: Prerelease::default(),
        };
        let mut comparators: Vec<Comparator> = Vec::new();
        comparators.push(uc);
        Ok(Import {
            name: format!("{namespace}:{name}"),
            req: VersionReq { comparators },
        })
    }
}
/// Builds list of packages in order that they should be added to locked component
pub struct LockListBuilder {
    // lock_list: IndexSet<String>,
    lock_list: IndexSet<Import>,
}

impl LockListBuilder {
    /// New LockListBuilder
    fn new() -> Self {
        Self {
            lock_list: IndexSet::new(),
        }
    }

    fn parse_import(
        &mut self,
        parser: &ComponentImportSectionReader,
        imports: &mut Vec<String>,
    ) -> Result<()> {
        let clone = parser.clone();
        for import in clone.into_iter_with_offsets() {
            let (_, imp) = import?;
            dbg!(&imp.name);
            imports.push(imp.name.0.to_string());
            // let kindless_name = imp.name.0.splitn(2, '=').last();
            // if let Some(name) = kindless_name {
            //     imports.push(name.to_string());
            // }
        }
        Ok(())
    }

    #[async_recursion]
    async fn parse_package(&mut self, client: &FileSystemClient, mut bytes: &[u8]) -> Result<()> {
        let mut parser = Parser::new(0);
        let mut imports: Vec<String> = Vec::new();
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
                    self.parse_import(&s, &mut imports)?;
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
        for import in imports {
            let mut resolver = ResolutionParser {
                next: &import,
                offset: 0,
            };

            let import = resolver.parse()?;
            // let pkg_name = import.split('/').next();

            // if let Some(name) = pkg_name {
            // let mut name_and_version = name.split('@');
            // let identifier = name_and_version.next();
            // if let Some(pkg_name) = identifier {
            // let id = PackageName::new(pkg_name.replace('<', ""))?;
            let id = PackageName::new(import.name.clone())?;
            dbg!(&id);
            if let Some(info) = client.registry().load_package(&id).await? {
                let release = info.state.releases().last();
                if let Some(r) = release {
                    let state = &r.state;
                    if let ReleaseState::Released { content } = state {
                        let path = client.content().content_location(content);
                        if let Some(p) = path {
                            let bytes = fs::read(p)?;
                            self.parse_package(client, &bytes).await?;
                        }
                    }
                }
                // self.lock_list.insert(name.replace('<', "").to_string());
                // self.lock_list.insert(import.name);
                self.lock_list.insert(import);
            } else {
                dbg!(&id);
                client.download(&id, &VersionReq::STAR).await?;
                if let Some(info) = client.registry().load_package(&id).await? {
                    let release = info.state.releases().last();
                    if let Some(r) = release {
                        let state = &r.state;
                        if let ReleaseState::Released { content } = state {
                            let path = client.content().content_location(content);
                            if let Some(p) = path {
                                let bytes = fs::read(p)?;
                                self.parse_package(client, &bytes).await?;
                            }
                        }
                    }
                    // self.lock_list.insert(name.replace('<', "").to_string());
                    // self.lock_list.insert(import.name);
                    self.lock_list.insert(import);
                }
            }
            // }
            // }
        }
        Ok(())
    }

    #[async_recursion]
    async fn build_list(&mut self, client: &FileSystemClient, info: &PackageInfo) -> Result<()> {
        let release = info.state.releases().last();
        if let Some(r) = release {
            let state = &r.state;
            if let ReleaseState::Released { content } = state {
                let path = client.content().content_location(content);
                if let Some(p) = path {
                    let bytes = fs::read(p)?;
                    self.parse_package(client, &bytes).await?;
                }
            }
        }
        Ok(())
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
    pub package: Option<PackageName>,

    /// Only show information for the specified package.
    #[clap(long = "exec", value_name = "EXEC", required = false)]
    pub executable: bool,
}

impl LockCommand {
    /// Executes the command.
    pub async fn exec(self) -> Result<()> {
        let config = self.common.read_config()?;
        let client = self.common.create_client(&config)?;
        println!("registry: {url}", url = client.url());
        println!("\npackages in client storage:");
        if let Some(package) = self.package {
            if let Some(info) = client.registry().load_package(&package).await? {
                Self::lock(client, &info, self.executable).await?;
            } else {
                client.download(&package, &VersionReq::STAR).await?;
                if let Some(info) = client.registry().load_package(&package).await? {
                    Self::lock(client, &info, self.executable).await?;
                }
            }
        }
        Ok(())
    }

    async fn lock(client: FileSystemClient, info: &PackageInfo, should_bundle: bool) -> Result<()> {
        let maybe_find = |s: &str, c: char| s.find(c);
        let mut builder = LockListBuilder::new();
        builder.build_list(&client, info).await?;
        // let parser = ResolutionParser {next: info.name, }
        let top = Import {
            name: format!("{}:{}", info.name.namespace(), info.name.name()),
            req: VersionReq::STAR,
        };
        builder.lock_list.insert(top);
        // builder
        //     .lock_list
        //     .insert(format!("{}:{}", info.name.namespace(), info.name.name()));
        dbg!(&builder.lock_list);
        let mut composer = CompositionGraph::new();
        let mut handled = HashMap::<String, InstanceId>::new();
        for package in builder.lock_list {
            // let mut name_and_version = package.split('@');
            // let name = name_and_version.next();
            let name = package.name.clone();
            // let version = name_and_version.next();
            let version = package.req;
            // if let Some(pkg_id) = name {
            // let id = PackageName::new(pkg_id.replace('<', ""))?;
            let id = PackageName::new(name)?;
            let info = client.registry().load_package(&id).await?;
            if let Some(inf) = info {
                // let release = if let Some(v) = version {
                //         let mut iterable = v.chars();
                //         let (lower, upper) = match iterable.next() {
                //             Some('{') => {
                //                 match iterable.next() {
                //                     Some('>') => match iterable.next() {
                //                         Some('=') => {
                //                             let space = maybe_find(v, ' ');
                //                             if let Some(sp) = space {
                //                                 let lower_bound = &v[3..sp];
                //                                 let lversion = lower_bound.parse::<Version>()?;
                //                                 let close = maybe_find(v, '}');
                //                                 if let Some(c) = close {
                //                                     let upper_bound = &v[sp + 2..c];
                //                                     let rversion = upper_bound.parse::<Version>()?;
                //                                     (Some(lversion), Some(rversion))
                //                                 } else {
                //                                     bail!("Range specification missing closing curly brace");
                //                                 }
                //                             } else {
                //                                 let close = maybe_find(v, '}');
                //                                 if let Some(c) = close {
                //                                     let lower_bound = &v[3..c];
                //                                     let version = lower_bound.parse::<Version>()?;
                //                                     (Some(version), None)
                //                                 } else {
                //                                     bail!("Range specification missing closing curly brace");
                //                                 }
                //                             }
                //                         }
                //                         _ => bail!("Lower version bound must be inclusive"),
                //                     },
                //                     Some('<') => {
                //                         let close = maybe_find(v, '}');
                //                         if let Some(c) = close {
                //                             let upper_bound = &v[3..c];
                //                             let version = upper_bound.parse::<Version>()?;
                //                             (None, Some(version))
                //                         } else {
                //                             bail!("Range specification missing closing curly brace");
                //                         }
                //                     }
                //                     _ => {
                //                         bail!("Invalid version specification, curly brace usage implies a range should be specified")
                //                     }
                //                 }
                //             }
                //             _ => (None, None),
                //         };
                //         match (lower, upper) {
                //             (Some(l), Some(u)) => {
                //                 let req = VersionReq::parse(&format!(">={}, <{}", l, u))?;
                //                 let matches = inf.state.releases().filter(|r| req.matches(&r.version));
                //                 matches.last()
                //             }
                //             (None, Some(u)) => {
                //                 let req = VersionReq::parse(&format!("<{}", u))?;
                //                 let matches = inf.state.releases().filter(|r| req.matches(&r.version));
                //                 matches.last()
                //             }
                //             (Some(l), None) => {
                //                 let req = VersionReq::parse(&format!(">={}", l))?;
                //                 let matches = inf.state.releases().filter(|r| req.matches(&r.version));
                //                 matches.last()
                //             }
                //             (None, None) => inf.state.releases().last(),
                //         }
                // } else {
                let release = inf.state.releases().last();
                // };
                // let release = info.state.releases().last();
                if let Some(r) = release {
                    let state = &r.state;
                    if let ReleaseState::Released { content } = state {
                        // let mut locked_package = package.split('@').next().unwrap().to_string();
                        let mut locked_package = package.name.clone();
                        locked_package = format!(
                            "locked-dep=<{}@{}>,integrity=<{}>",
                            locked_package,
                            &r.version.to_string(),
                            content.to_string().replace(':', "-")
                        );
                        let path = client.content().content_location(content);
                        if let Some(p) = path {
                            let component =
                                wasm_compose::graph::Component::from_file(&locked_package, p)?;
                            let component_index =
                                if let Some(c) = composer.get_component_by_name(&locked_package) {
                                    c.0
                                } else {
                                    composer.add_component(component)?
                                };
                            let instance_id = composer.instantiate(component_index)?;
                            let added = composer.get_component(component_index);
                            handled.insert(package.name, instance_id);
                            let mut args = Vec::new();
                            if let Some(added) = added {
                                for (index, name, _) in added.imports() {
                                    let kindless_name = name.splitn(2, '=').last();
                                    if let Some(name) = kindless_name {
                                        let iid = handled.get(&name.replace('<', ""));
                                        if let Some(arg) = iid {
                                            args.push((arg, index));
                                        }
                                    }
                                }
                            }
                            for arg in args {
                                composer.connect(
                                    *arg.0,
                                    None::<ExportIndex>,
                                    instance_id,
                                    arg.1,
                                )?;
                            }
                        }
                    }
                }
            }
            // }
        }
        let final_name = &format!("{}:{}", info.name.namespace(), &info.name.name());

        let id = handled.get(final_name);
        let options = if let Some(id) = id {
            EncodeOptions {
                define_components: should_bundle,
                export: Some(*id),
                validate: false,
            }
        } else {
            EncodeOptions {
                define_components: false,
                export: None,
                validate: false,
            }
        };
        let locked = composer.encode(options)?;
        fs::write("./locked.wasm", locked.as_slice())?;
        Ok(())
    }
}
