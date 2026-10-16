//! frob:ticket 01M4TA3CCFZS5X2P2YESK4E68X
//! The `std` package compiled into the binary, so `cadetta` needs no `std/` on disk.

/// Every `std/src` file as (path under `src`, text); kept in step with `std/` by a test.
pub const STD_FILES: &[(&str, &str)] = &[
    (
        "commercial.cdt",
        include_str!("../../../std/src/commercial.cdt"),
    ),
    (
        "fasteners.cdt",
        include_str!("../../../std/src/fasteners.cdt"),
    ),
    (
        "fasteners/inch.cdt",
        include_str!("../../../std/src/fasteners/inch.cdt"),
    ),
    (
        "fasteners/policy.cdt",
        include_str!("../../../std/src/fasteners/policy.cdt"),
    ),
    (
        "fasteners/screw.cdt",
        include_str!("../../../std/src/fasteners/screw.cdt"),
    ),
    (
        "fasteners/screwjoint.cdt",
        include_str!("../../../std/src/fasteners/screwjoint.cdt"),
    ),
    (
        "fasteners/tables.cdt",
        include_str!("../../../std/src/fasteners/tables.cdt"),
    ),
    ("fits.cdt", include_str!("../../../std/src/fits.cdt")),
    ("inserts.cdt", include_str!("../../../std/src/inserts.cdt")),
    ("lib.cdt", include_str!("../../../std/src/lib.cdt")),
    (
        "materials.cdt",
        include_str!("../../../std/src/materials.cdt"),
    ),
    (
        "retaining.cdt",
        include_str!("../../../std/src/retaining.cdt"),
    ),
];

#[cfg(test)]
mod tests {
    use super::STD_FILES;
    use std::path::Path;

    fn disk_files(base: &Path, dir: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("read std dir") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                disk_files(base, &path, out);
            } else if path.extension().is_some_and(|e| e == "cdt") {
                let rel = path.strip_prefix(base).expect("under base");
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    // frob:tests embedded_std_matches_std_directory
    #[test]
    fn embedded_std_matches_std_directory() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../std/src");
        let mut on_disk = Vec::new();
        disk_files(&base, &base, &mut on_disk);
        on_disk.sort();
        let mut embedded: Vec<String> = STD_FILES.iter().map(|(p, _)| (*p).to_owned()).collect();
        embedded.sort();
        assert_eq!(embedded, on_disk, "STD_FILES must list every std/src file");
        for (rel, text) in STD_FILES {
            let disk = std::fs::read_to_string(base.join(rel)).expect("read");
            assert_eq!(&disk, text, "{rel} differs");
        }
    }
}
