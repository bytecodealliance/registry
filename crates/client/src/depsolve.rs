use anyhow::{bail, Result};
use async_recursion::async_recursion;
use indexmap::IndexSet;
use semver::{Comparator, Prerelease, Version, VersionReq};
use std::fs;
use warg_protocol::package::ReleaseState;
use warg_protocol::registry::PackageName;
use wasm_encoder::{
    Component, ComponentImportSection, ComponentSectionId, ComponentTypeRef, RawSection,
};
use wasmparser::names::KebabStr;
use wasmparser::{Chunk, ComponentImportSectionReader, Parser, Payload};

use super::Client;
use crate::storage::{ContentStorage, PackageInfo, RegistryStorage};
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

/// Parser for dep solve deps
pub struct DependencyImportParser<'a> {
    /// string to be parsed
    pub next: &'a str,
    /// index of parser
    pub offset: usize,
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

/// Creates list of dependenies for locking components
pub struct LockListBuilder {
    /// List of deps to include in locked component
    pub lock_list: IndexSet<Import>,
}

impl LockListBuilder {
    /// New LockListBuilder
    pub fn new() -> Self {
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
    async fn parse_package<R: RegistryStorage, C: ContentStorage>(
        &mut self,
        client: &Client<R, C>,
        mut bytes: &[u8],
    ) -> Result<()> {
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

    /// List of deps for building
    #[async_recursion]
    pub async fn build_list<R: RegistryStorage, C: ContentStorage>(
        &mut self,
        client: &Client<R, C>,
        info: &PackageInfo,
    ) -> Result<()> {
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

/// Bundles Dependencies
pub struct Bundler<'a, R, C>
where
    R: RegistryStorage,
    C: ContentStorage,
{
    /// Warg client used for bundling
    client: &'a Client<R, C>,
}

impl<'a, R, C> Bundler<'a, R, C>
where
    R: RegistryStorage,
    C: ContentStorage,
{
    /// New Bundler
    pub fn new(client: &'a Client<R, C>) -> Self {
        Self { client }
    }

    async fn parse_imports(
        &mut self,
        parser: ComponentImportSectionReader<'a>,
        component: &mut Component,
    ) -> Result<Vec<u8>> {
        let mut imports = ComponentImportSection::new();
        for import in parser.into_iter_with_offsets() {
            let (_, imp) = import?;
            let mut dep_parser = DependencyImportParser {
                next: imp.name.0,
                offset: 0,
            };
            let parsed_imp = dep_parser.parse()?;
            if !parsed_imp.name.contains('/') {
                let pkg_id = PackageName::new(parsed_imp.name)?;
                if let Some(info) = self.client.registry().load_package(&pkg_id).await? {
                    let release = if parsed_imp.req != VersionReq::STAR {
                        info.state
                            .releases()
                            .filter(|r| parsed_imp.req.matches(&r.version))
                            .last()
                    } else {
                        info.state.releases().last()
                    };
                    if let Some(r) = release {
                        let release_state = &r.state;
                        if let ReleaseState::Released { content } = release_state {
                            let path = self.client.content().content_location(content);
                            if let Some(p) = path {
                                let bytes = fs::read(p)?;
                                component.section(&RawSection {
                                    id: ComponentSectionId::Component.into(),
                                    data: &bytes,
                                });
                            }
                        }
                    }
                }
            } else if let wasmparser::ComponentTypeRef::Instance(i) = imp.ty {
                imports.import(imp.name.0, ComponentTypeRef::Instance(i));
            }
        }
        component.section(&imports);
        Ok(Vec::new())
    }

    /// Parse bytes for bundling
    pub async fn parse(&mut self, mut bytes: &'a [u8]) -> Result<Component> {
        let constant = bytes.clone();
        let mut parser = Parser::new(0);
        let mut component = Component::new();
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
                    self.parse_imports(s, &mut component).await?;
                }
                Payload::ModuleSection { range, .. } => {
                    let offset = range.end - range.start;
                    component.section(&RawSection {
                        id: 1,
                        data: &constant[range],
                    });
                    if offset > bytes.len() {
                        panic!();
                    }
                    bytes = &bytes[offset..];
                }
                Payload::End(_) => {
                    break;
                }
                _ => {
                    if let Some((id, range)) = payload.as_section() {
                        component.section(&RawSection {
                            id,
                            data: &constant[range],
                        });
                    }
                }
            }
        }
        Ok(component)
    }
}
