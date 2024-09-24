use anyhow::{bail, Result};
use async_recursion::async_recursion;
use indexmap::IndexSet;
use semver::VersionReq;
use std::fs;
use warg_protocol::package::{Release, ReleaseState};
use warg_protocol::registry::PackageName;
use wasm_encoder::{
    Component, ComponentImportSection, ComponentSectionId, ComponentTypeRef, RawSection,
};
use wasmparser::{Chunk, ComponentImportSectionReader, Parser, Payload};

use super::Client;
use crate::storage::{ContentStorage, NamespaceMapStorage, PackageInfo, RegistryStorage};
use crate::version_util::{DependencyImportParser, Import, ImportKind};
/// Import Kinds found in components

/// Creates list of dependenies for locking components
pub struct LockListBuilder {
    /// List of deps to include in locked component
    pub lock_list: IndexSet<Import>,
}

impl Default for LockListBuilder {
    /// New LockListBuilder
    fn default() -> Self {
        Self {
            lock_list: IndexSet::new(),
        }
    }
}

impl LockListBuilder {
    fn parse_import(
        &self,
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
    async fn parse_package<R, C, N>(
        &mut self,
        client: &Client<R, C, N>,
        mut bytes: &[u8],
    ) -> Result<()>
    where
        R: RegistryStorage,
        C: ContentStorage,
        N: NamespaceMapStorage,
    {
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
                    let registry_domain = client.get_warg_registry(id.namespace()).await?;
                    if let Some(info) = client
                        .registry()
                        .load_package(registry_domain.as_ref(), &id)
                        .await?
                    {
                        let release = info.state.releases().last();
                        if let Some(r) = release {
                            if let Some(bytes) = self.release_bytes(r, client)? {
                                self.parse_package(client, &bytes).await?;
                            }
                        }
                        self.lock_list.insert(import);
                    } else {
                        client.download(&id, &VersionReq::STAR, |_, _| ()).await?;
                        if let Some(info) = client
                            .registry()
                            .load_package(
                                client.get_warg_registry(id.namespace()).await?.as_ref(),
                                &id,
                            )
                            .await?
                        {
                            let release = info.state.releases().last();
                            if let Some(r) = release {
                                if let Some(bytes) = self.release_bytes(r, client)? {
                                    self.parse_package(client, &bytes).await?;
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

    fn release_bytes<R: RegistryStorage, C: ContentStorage, N: NamespaceMapStorage>(
        &self,
        release: &Release,
        client: &Client<R, C, N>,
    ) -> Result<Option<Vec<u8>>> {
        let state = &release.state;
        if let ReleaseState::Released { content } = state {
            let path = client.content().content_location(content);
            if let Some(p) = path {
                return Ok(Some(fs::read(p)?));
            }
        }
        Ok(None)
    }

    /// List of deps for building
    #[async_recursion]
    pub async fn build_list<R, C, N>(
        &mut self,
        client: &Client<R, C, N>,
        info: &PackageInfo,
    ) -> Result<()>
    where
        R: RegistryStorage,
        C: ContentStorage,
        N: NamespaceMapStorage,
    {
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
pub struct Bundler<'a, R, C, N>
where
    R: RegistryStorage,
    C: ContentStorage,
    N: NamespaceMapStorage,
{
    /// Warg client used for bundling
    client: &'a Client<R, C, N>,
}

impl<'a, R, C, N> Bundler<'a, R, C, N>
where
    R: RegistryStorage,
    C: ContentStorage,
    N: NamespaceMapStorage,
{
    /// New Bundler
    pub fn new(client: &'a Client<R, C, N>) -> Self {
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
                if let Some(info) = self
                    .client
                    .registry()
                    .load_package(
                        self.client
                            .get_warg_registry(pkg_id.namespace())
                            .await?
                            .as_ref(),
                        &pkg_id,
                    )
                    .await?
                {
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
        let constant = bytes;
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
