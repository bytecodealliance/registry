use super::CommonOptions;
use anyhow::{bail, Result};
use async_recursion::async_recursion;
use clap::Args;
use indexmap::IndexSet;
use semver::{Comparator, Prerelease, Version, VersionReq};
use sha256;
use std::{collections::HashMap, fs};
use warg_client::{
    storage::{ContentStorage, PackageInfo, RegistryStorage},
    FileSystemClient,
};
use warg_protocol::{package::ReleaseState, registry::PackageName};
use wasm_compose::graph::{CompositionGraph, EncodeOptions, ExportIndex, InstanceId};
use wasmparser::{names::KebabStr, Chunk, ComponentImportSectionReader, Parser, Payload};

/// Parser for dep solve deps
pub struct DependencyImportParser<'a> {
    /// string to be parsed
    pub next: &'a str,
    /// index of parser
    pub offset: usize,
}

/// Import Kinds found in components
#[derive(Debug, Eq, PartialEq, Hash)]
pub enum ImportKind {
    /// Locked Version
    Locked(Option<String>),
    /// Unlocked Version Range
    Unlocked,
    /// Interface
    Interface(Option<String>),
}

/// Dependency in dep solve
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Import {
    /// Import name
    pub name: String,
    /// Version Requirements
    pub req: VersionReq,
    /// Import kind
    pub kind: ImportKind,
}

impl<'a> DependencyImportParser<'a> {
    /// Parses import
    pub fn parse(&mut self) -> Result<Import> {
        if self.eat_str("unlocked-dep=") {
            self.expect_str("<")?;
            let imp = self.pkgidset_up_to('>')?;
            self.expect_str(">")?;
            return Ok(imp);
        }

        if self.eat_str("locked-dep=") {
            self.expect_str("<")?;
            let imp = self.pkgver()?;
            return Ok(imp);
        }

        let name = self.eat_until('@');
        let v = self.semver(self.next)?;
        let comp = Comparator {
            op: semver::Op::Exact,
            major: v.major,
            minor: Some(v.minor),
            patch: Some(v.patch),
            pre: v.pre,
        };
        let req = VersionReq {
            comparators: vec![comp],
        };
        Ok(Import {
            name: name.unwrap().to_string(),
            req,
            kind: ImportKind::Interface(Some(self.next.to_string())),
        })
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

    fn pkgver(&mut self) -> Result<Import> {
        let namespace = self.take_until(':')?;
        self.kebab(namespace)?;
        let name = match self.eat_until('@') {
            Some(name) => name,
            // a:b
            None => {
                let name = self.take_up_to(',')?;
                self.kebab(name)?;
                return Ok(Import {
                    name: format!("{namespace}:{name}"),
                    req: VersionReq::STAR,
                    kind: ImportKind::Locked(None),
                });
            }
        };
        let version = self.eat_until('>');
        let req = if let Some(v) = version {
            let v = self.semver(v)?;
            let comp = Comparator {
                op: semver::Op::Exact,
                major: v.major,
                minor: Some(v.minor),
                patch: Some(v.patch),
                pre: Prerelease::default(),
            };
            VersionReq {
                comparators: vec![comp],
            }
        } else {
            VersionReq::STAR
        };
        let digest = if self.eat_str(",") {
            self.eat_until('<');
            self.eat_until('>').map(|d| d.to_string())
        } else {
            None
        };
        Ok(Import {
            name: format!("{namespace}:{name}"),
            req,
            kind: ImportKind::Locked(digest),
        })
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
                    kind: ImportKind::Unlocked,
                });
            }
        };
        self.kebab(name)?;
        // a:b@*
        if self.eat_str("*") {
            return Ok(Import {
                name: format!("{namespace}:{name}"),
                req: VersionReq::STAR,
                kind: ImportKind::Unlocked,
            });
        }
        self.expect_str("{")?;
        if self.eat_str(">=") {
            match self.eat_until(' ') {
                Some(lower) => {
                    let lower = self.semver(lower)?;
                    self.expect_str("<")?;
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
                    let comparators = vec![lc, uc];
                    return Ok(Import {
                        name: format!("{namespace}:{name}"),
                        req: VersionReq { comparators },
                        kind: ImportKind::Unlocked,
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
                    let comparators = vec![comparator];
                    return Ok(Import {
                        name: format!("{namespace}:{name}"),
                        req: VersionReq { comparators },
                        kind: ImportKind::Unlocked,
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
            kind: ImportKind::Unlocked,
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
            imports.push(imp.name.0.to_string());
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
            let mut resolver = DependencyImportParser {
                next: &import,
                offset: 0,
            };

            let import = resolver.parse()?;
            match import.kind {
                ImportKind::Locked(_) | ImportKind::Unlocked => {
                    let id = PackageName::new(import.name.clone())?;
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
                        self.lock_list.insert(import);
                    } else {
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
                            self.lock_list.insert(import);
                        }
                    }
                }
                ImportKind::Interface(_) => {}
            }
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
        let mut builder = LockListBuilder::new();
        builder.build_list(&client, info).await?;
        let top = Import {
            name: format!("{}:{}", info.name.namespace(), info.name.name()),
            req: VersionReq::STAR,
            kind: ImportKind::Unlocked,
        };
        builder.lock_list.insert(top);
        let mut composer = CompositionGraph::new();
        let mut handled = HashMap::<String, InstanceId>::new();
        for package in builder.lock_list {
            let name = package.name.clone();
            let version = package.req;
            let id = PackageName::new(name)?;
            let info = client.registry().load_package(&id).await?;
            if let Some(inf) = info {
                let release = if version != VersionReq::STAR {
                    inf.state
                        .releases()
                        .filter(|r| version.matches(&r.version))
                        .last()
                } else {
                    inf.state.releases().last()
                };

                if let Some(r) = release {
                    let state = &r.state;
                    if let ReleaseState::Released { content } = state {
                        let mut locked_package = package.name.clone();
                        locked_package = format!(
                            "locked-dep=<{}@{}>,integrity=<{}>",
                            locked_package,
                            &r.version.to_string(),
                            content.to_string().replace(':', "-")
                        );
                        let path = client.content().content_location(content);
                        if let Some(p) = path {
                            let read_digest = sha256::try_digest(&p)?;
                            if content.to_string().split(':').last().unwrap() != read_digest {
                                bail!("Expected content digest to be `{content}`, instead found `{read_digest}`");
                            }
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
                            let ver = version.clone().to_string();
                            let range = if ver == "*" {
                                "".to_string()
                            } else {
                                format!("@{{{}}}", ver.replace(',', ""))
                            };
                            handled.insert(format!("{}{range}", package.name), instance_id);
                            let mut args = Vec::new();
                            if let Some(added) = added {
                                for (index, name, _) in added.imports() {
                                    let kindless_name = name.splitn(2, '=').last();
                                    if let Some(name) = kindless_name {
                                        let iid = handled.get(&name[1..name.len() - 1]);
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
