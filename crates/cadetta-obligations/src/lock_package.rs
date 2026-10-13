//! `package` lock records: content digests of dependencies, E014 and closure inputs.
//!
//! Specification: `docs/spec/packages.md` sections 1 and 6, `docs/spec/obligations.md` 3.
//! frob:ticket 01M4SXR720K3VGM1K7DWZ0VJ7Z

use std::fmt;

use cadetta_ir::digest::{DigestGraph, digest_bytes};
use cadetta_ir::model::Digest;
use tracing::{debug, info, warn};

use crate::lock::{Hex, Lock, Package, Record};

/// Domain tag mixed into every package content digest.
const TAG: &str = "package-content";

/// A resolved dependency as the resolver reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPackage {
    /// Package name.
    pub name: String,
    /// Resolved version.
    pub version: String,
    /// Source locator (path, or git url and revision).
    pub source: String,
    /// Content digest from [`content_digest`].
    pub digest: Hex,
}

/// Digest dependency content given as (path relative to the package root, bytes) pairs.
///
/// Files are sorted by relative path with `/` separators and length-prefixed, so the
/// digest never depends on the checkout location or on enumeration order (ir.md 4).
pub fn content_digest<P: AsRef<str>, B: AsRef<[u8]>>(files: &[(P, B)]) -> Hex {
    let mut entries: Vec<(String, &[u8])> = files
        .iter()
        .map(|(p, b)| (p.as_ref().replace('\\', "/"), b.as_ref()))
        .collect();
    entries.sort();
    let mut buf = Vec::new();
    let mut field = |bytes: &[u8]| {
        buf.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(bytes);
    };
    field(TAG.as_bytes());
    for (path, bytes) in &entries {
        field(path.as_bytes());
        field(bytes);
    }
    let d = digest_bytes(&buf);
    debug!(files = entries.len(), digest = %d.0, "package content digested");
    Hex::new(&d.0).expect("blake3 hex is lowercase hex")
}

/// Record `pkg` in the lock, replacing any older pin of the same name (`cadetta update`).
pub fn pin_package(lock: &mut Lock, pkg: &ResolvedPackage) {
    let rec = Record::Package(Package {
        name: pkg.name.clone(),
        version: pkg.version.clone(),
        source: pkg.source.clone(),
        digest: pkg.digest.clone(),
    });
    lock.remove(&rec.key());
    lock.insert(rec);
    info!(name = %pkg.name, version = %pkg.version, "package pinned");
}

/// E014: a dependency's content differs from the digest pinned in the lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyChanged {
    /// Package name.
    pub name: String,
    /// Digest in the lock.
    pub pinned: Hex,
    /// Digest of the content now.
    pub current: Hex,
}

impl DependencyChanged {
    /// The diagnostic code.
    pub const CODE: &'static str = "E014";
}

impl fmt::Display for DependencyChanged {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: dependency `{}` changed: pinned digest {} but content is {}; run `cadetta update {}`",
            Self::CODE,
            self.name,
            self.pinned.as_str(),
            self.current.as_str(),
            self.name,
        )
    }
}

/// Check resolved dependencies against their pinned records; unpinned ones are skipped.
pub fn check_pinned(lock: &Lock, resolved: &[ResolvedPackage]) -> Vec<DependencyChanged> {
    let mut out = Vec::new();
    for pkg in resolved {
        let key = ("package", vec![pkg.name.clone()]);
        match lock.get(&key) {
            Some(Record::Package(p)) if p.digest != pkg.digest => {
                warn!(name = %pkg.name, "E014 dependency content changed");
                out.push(DependencyChanged {
                    name: pkg.name.clone(),
                    pinned: p.digest.clone(),
                    current: pkg.digest.clone(),
                });
            }
            Some(_) => debug!(name = %pkg.name, "dependency matches its pin"),
            None => debug!(name = %pkg.name, "dependency not pinned"),
        }
    }
    out
}

/// Graph key of a package node; readers list it among their deps (packages.md 6).
pub fn node_key(name: &str) -> String {
    format!("package:{name}")
}

/// Add the package as a graph node whose own digest is its content digest.
pub fn add_to_graph(graph: &mut DigestGraph, pkg: &ResolvedPackage) {
    graph.add_node(
        node_key(&pkg.name),
        Digest(pkg.digest.as_str().to_owned()),
        Vec::new(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(name: &str, files: &[(&str, &str)]) -> ResolvedPackage {
        ResolvedPackage {
            name: name.into(),
            version: "1.0.0".into(),
            source: "path:../x".into(),
            digest: content_digest(files),
        }
    }

    // frob:tests crates/cadetta-obligations/src/lock_package.rs::pin_package
    #[test]
    fn pin_writes_package_line_and_replaces() {
        let mut lock = Lock::new();
        let p = pkg("acme", &[("src/lib.cdt", "a")]);
        pin_package(&mut lock, &p);
        pin_package(&mut lock, &pkg("acme", &[("src/lib.cdt", "b")]));
        pin_package(&mut lock, &pkg("abc", &[]));
        assert_eq!(lock.len(), 2);
        let text = lock.render();
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with("package abc version=1.0.0 source=path:../x digest="));
        assert!(lines[1].starts_with("package acme "));
        assert_eq!(Lock::parse(&text).unwrap().render(), text);
    }

    // frob:tests crates/cadetta-obligations/src/lock_package.rs::check_pinned
    #[test]
    fn changed_content_is_e014_naming_update() {
        let mut lock = Lock::new();
        pin_package(&mut lock, &pkg("acme", &[("a", "1")]));
        assert!(check_pinned(&lock, &[pkg("acme", &[("a", "1")])]).is_empty());
        let found = check_pinned(&lock, &[pkg("acme", &[("a", "2")])]);
        assert_eq!(found.len(), 1);
        let msg = found[0].to_string();
        assert!(msg.starts_with("E014"));
        assert!(msg.contains("cadetta update"));
    }

    // frob:tests crates/cadetta-obligations/src/lock_package.rs::content_digest
    #[test]
    fn digest_ignores_order_and_separator_not_content() {
        let a = content_digest(&[("src/a.cdt", "x"), ("b.cdt", "y")]);
        let b = content_digest(&[("b.cdt", "y"), ("src\\a.cdt", "x")]);
        assert_eq!(a, b);
        assert_ne!(a, content_digest(&[("src/a.cdt", "x"), ("b.cdt", "z")]));
        assert_ne!(a, content_digest(&[("src/a.cdt", "xb.cdt"), ("", "y")]));
    }

    // frob:tests crates/cadetta-obligations/src/lock_package.rs::add_to_graph
    #[test]
    fn dependency_digest_enters_reader_closure() {
        let closure = |content: &str| {
            let mut g = DigestGraph::new();
            add_to_graph(&mut g, &pkg("acme", &[("a", content)]));
            g.add_node("app::m", Digest("00".into()), vec![node_key("acme")]);
            g.closure("app::m").unwrap()
        };
        assert_eq!(closure("1"), closure("1"));
        assert_ne!(closure("1"), closure("2"));
    }
}
