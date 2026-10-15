//! File-tree modules, canonical item paths and E015 (packages.md 2, config-cli.md 1.2-1.4).
//!
//! The module tree is the file tree under `src`. File access is injected: the
//! caller lists the `src`-relative files and this module maps them to modules,
//! so loading is pure and deterministic. Decisions written into packages.md 2:
//! only `*.cdt` files are modules (other files are ignored); every path
//! component (directories and file stems) must be `snake_case`; a missing
//! parent file (`drive.cdt` beside `drive/`) is fine and the parent module
//! still exists implicitly; an empty or missing `src` is E005.

// frob:ticket 01M4SXRFTTTTJE5CHCZAE8MX0Q

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Component, Path, PathBuf};

use tracing::{debug, trace, warn};

use crate::resolve::{CanonPath, MemberSeg};

/// A module-loading diagnostic (E005 or E015) tied to a `src`-relative file.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ModuleError {
    /// The catalog code, `E005` or `E015`.
    pub code: &'static str,
    /// The message.
    pub message: String,
    /// The offending `src`-relative path, empty for a package-wide error.
    pub file: String,
}

impl fmt::Display for ModuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ModuleError {}

/// One module of a package: its path below the root and the file that holds it.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct ModuleFile {
    /// Module path below the package root (empty for `lib.cdt`).
    pub path: Vec<String>,
    /// The `src`-relative file with `/` separators, empty when the module is implicit.
    pub file: String,
}

impl ModuleFile {
    /// Renders `pkg::a::b`, or just `pkg` for the root.
    pub fn display(&self, package: &str) -> String {
        let mut s = package.to_owned();
        for seg in &self.path {
            s.push_str("::");
            s.push_str(seg);
        }
        s
    }
}

/// True for `snake_case` module names (N001): lowercase letters and digits, `_`-separated.
pub fn is_module_name(s: &str) -> bool {
    let mut parts = s.split('_');
    let first_ok = s.starts_with(|c: char| c.is_ascii_lowercase());
    first_ok
        && parts.all(|p| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

/// Maps one `src`-relative `.cdt` path to its module path, or E015.
pub fn module_path_of(rel: &str) -> Result<Vec<String>, ModuleError> {
    let stem = rel.strip_suffix(".cdt").unwrap_or(rel);
    if stem == "lib" {
        return Ok(Vec::new());
    }
    let segs: Vec<&str> = stem.split('/').collect();
    if let Some(bad) = segs.iter().find(|s| !is_module_name(s)) {
        warn!(file = rel, segment = *bad, "bad module name");
        return Err(ModuleError {
            code: "E015",
            message: format!("`{bad}` in `src/{rel}` is not a valid module name (snake_case)"),
            file: rel.to_owned(),
        });
    }
    Ok(segs.into_iter().map(str::to_owned).collect())
}

/// Builds the module list of a package from its `src`-relative file list.
///
/// Files that are not `.cdt` are ignored; input is sorted by path first so the
/// output (and any digest over it) is deterministic. Parents that have only a
/// directory (`drive/motor.cdt` without `drive.cdt`) are added as implicit
/// modules with an empty `file`. Returns every E015 found plus E005 when no
/// `.cdt` file exists.
pub fn load_modules(src_files: &[&str]) -> (Vec<ModuleFile>, Vec<ModuleError>) {
    let mut files: Vec<&str> = src_files
        .iter()
        .copied()
        .filter(|f| f.ends_with(".cdt"))
        .collect();
    files.sort_unstable();
    files.dedup();
    let mut errors = Vec::new();
    let mut mods: Vec<ModuleFile> = Vec::new();
    for f in &files {
        match module_path_of(f) {
            Ok(path) => {
                trace!(file = *f, ?path, "module");
                mods.push(ModuleFile {
                    path,
                    file: (*f).to_owned(),
                });
            }
            Err(e) => errors.push(e),
        }
    }
    if files.is_empty() {
        warn!("package has no .cdt file under src");
        errors.push(ModuleError {
            code: "E005",
            message: "`src` is missing or holds no `.cdt` file".to_owned(),
            file: String::new(),
        });
    }
    let have: BTreeSet<Vec<String>> = mods.iter().map(|m| m.path.clone()).collect();
    let mut implicit = BTreeSet::new();
    for m in &mods {
        for n in 1..m.path.len() {
            let parent = m.path[..n].to_vec();
            if !have.contains(&parent) {
                implicit.insert(parent);
            }
        }
    }
    mods.extend(implicit.into_iter().map(|path| ModuleFile {
        path,
        file: String::new(),
    }));
    mods.sort();
    debug!(
        modules = mods.len(),
        errors = errors.len(),
        "loaded module tree"
    );
    (mods, errors)
}

/// The canonical path `package::module::Item` of an item (ir.md 2).
pub fn item_path(package: &str, module: &[String], item: &str) -> CanonPath {
    CanonPath::item(package, module, item)
}

/// The canonical path of a member chain below an item, `Item.a.b`.
pub fn member_path(item: &CanonPath, members: &[&str]) -> CanonPath {
    let mut p = item.clone();
    p.members
        .extend(members.iter().map(|m| MemberSeg::Name((*m).to_owned())));
    p
}

/// Resolves a binding, solver script or cited-file path against the package directory, not `src`.
///
/// Returns `None` when the path is absolute or escapes the package directory.
pub fn resolve_package_file(package_dir: &Path, rel: &str) -> Option<PathBuf> {
    let mut out = package_dir.to_path_buf();
    let mut depth = 0usize;
    for c in Path::new(rel).components() {
        match c {
            Component::Normal(s) => {
                out.push(s);
                depth += 1;
            }
            Component::CurDir => {}
            Component::ParentDir if depth > 0 => {
                out.pop();
                depth -= 1;
            }
            _ => {
                warn!(rel, "file path escapes the package directory");
                return None;
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::{ModuleSrc, PackageSrc, Resolver};
    use cadetta_syntax::parser::parse;

    fn paths(mods: &[ModuleFile]) -> Vec<String> {
        mods.iter().map(|m| m.display("acme.rover")).collect()
    }

    // frob:tests crates/cadetta-sema/src/modules.rs::load_modules kind=unit
    #[test]
    fn tree_maps_files_to_modules_with_implicit_parent() {
        let (mods, errs) =
            load_modules(&["src_ignored.txt", "frame.cdt", "lib.cdt", "drive/motor.cdt"]);
        assert!(errs.is_empty());
        assert_eq!(
            paths(&mods),
            [
                "acme.rover",
                "acme.rover::drive",
                "acme.rover::drive::motor",
                "acme.rover::frame"
            ]
        );
        assert_eq!(mods[1].file, "");
    }

    // frob:tests crates/cadetta-sema/src/modules.rs::module_path_of kind=unit
    #[test]
    fn non_snake_case_is_e015() {
        for bad in [
            "Frame.cdt",
            "drive/Motor.cdt",
            "Drive/motor.cdt",
            "a__b.cdt",
            "1a.cdt",
            "a-b.cdt",
        ] {
            assert_eq!(module_path_of(bad).unwrap_err().code, "E015", "{bad}");
        }
        let (_, errs) = load_modules(&["lib.cdt", "BadName.cdt"]);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code, "E015");
        assert!(module_path_of("a_b2/c.cdt").is_ok());
    }

    // frob:tests crates/cadetta-sema/src/modules.rs::load_modules kind=unit
    #[test]
    fn empty_src_is_e005() {
        let (_, errs) = load_modules(&["readme.md"]);
        assert_eq!(errs[0].code, "E005");
    }

    // frob:tests crates/cadetta-sema/src/modules.rs::item_path kind=unit
    #[test]
    fn item_and_member_paths() {
        let item = item_path("acme.rover", &["drive".into(), "motor".into()], "Gm42A");
        let p = member_path(&item, &["rated_torque"]);
        assert_eq!(
            p.to_string(),
            "acme.rover::drive::motor::Gm42A.rated_torque"
        );
    }

    // frob:tests crates/cadetta-sema/src/modules.rs::load_modules kind=unit
    #[test]
    fn input_order_does_not_matter() {
        let a = load_modules(&["b.cdt", "lib.cdt", "a/x.cdt"]).0;
        let b = load_modules(&["a/x.cdt", "lib.cdt", "b.cdt"]).0;
        assert_eq!(a, b);
    }

    // frob:tests crates/cadetta-sema/src/modules.rs::load_modules kind=unit
    #[test]
    fn duplicate_component_in_a_module_is_e003_with_both_spans() {
        let parsed = parse("model A { x = 1 }\nmodel A { y = 1 }\n");
        let (_r, errs) = Resolver::new(vec![PackageSrc {
            defaults: Default::default(),
            name: "acme.rover".into(),
            deps: vec![],
            modules: vec![ModuleSrc {
                path: vec!["frame".into()],
                file: &parsed.file,
                enums: &[],
            }],
        }]);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code, "E003");
        assert_eq!(errs[0].related.len(), 1);
    }

    // frob:tests crates/cadetta-sema/src/modules.rs::resolve_package_file kind=unit
    #[test]
    fn files_resolve_against_package_dir() {
        let dir = Path::new("/w/acme");
        assert_eq!(
            resolve_package_file(dir, "bindings/sw.toml"),
            Some(PathBuf::from("/w/acme/bindings/sw.toml"))
        );
        assert_eq!(
            resolve_package_file(dir, "./docs/../docs/a.pdf"),
            Some(PathBuf::from("/w/acme/docs/a.pdf"))
        );
        assert_eq!(resolve_package_file(dir, "../x"), None);
        assert_eq!(resolve_package_file(dir, "/etc/x"), None);
    }
}
