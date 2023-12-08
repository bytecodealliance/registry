use anyhow::{bail, Result};
use semver::{Comparator, Op, Prerelease, Version, VersionReq};
use warg_crypto::hash::AnyHash;
use warg_protocol::package::Release;
use wasmparser::names::KebabStr;

/// Kind of import encountered while parsing
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

/// Returns locked package string
pub fn locked_package(pkg_name: &str, release: &Release, content: &AnyHash) -> String {
    format!(
        "locked-dep=<{}@{}>,integrity=<{}>",
        &pkg_name,
        &release.version.to_string(),
        &content.to_string().replace(':', "-")
    )
}

/// Package name with version range
pub fn versioned_package(pkg_name: &str, version: VersionReq) -> String {
    let ver = version.clone().to_string();
    let range = if ver == "*" {
        "".to_string()
    } else {
        // @{<verlower> <verupper>}
        format!("@{{{}}}", ver.replace(',', ""))
    };
    format!("{}{range}", pkg_name)
}

/// Remove import kind from import beginning
pub fn kindless_name(import_name: &str) -> &str {
    // unlocked-dep=<foo:bar@version> --> <foo:bar@version>
    let kindless_name = import_name.splitn(2, '=').last().unwrap();
    // remove angle brackets
    &kindless_name[1..kindless_name.len() - 1]
}

/// Stringify version
pub fn version_string(version: &VersionReq) -> String {
    if version.to_string() == "*" {
        "*".to_string()
    } else if version.comparators.len() == 1 && version.comparators[0].op == Op::Exact {
        version.to_string()
    } else {
        format!("{{{}}}", version.to_string().replace(',', ""))
    }
}
