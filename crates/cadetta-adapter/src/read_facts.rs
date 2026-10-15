//! Pure read-side checks over binding facts (adapters.md 2.2, 3.3, 3.4): R007 for a dimension
//! not measured from the sketch origin, R011 for a component instance whose file is not its
//! slot's file, and lock-digest reuse of an unchanged saved file.
// frob:ticket 01M4Z8V9VNHK5QN6G0B0MC1ZA9

use std::collections::BTreeMap;

use cadetta_ir::model::LocatorKind;

use crate::{AdapterDiagnostic, Severity};

/// Diagnostic code for a binding whose native datum or dimension disagrees with its design datum (geometry.md 6).
pub const R007: &str = "R007";
/// Diagnostic code for a component instance whose file is not the slot's file (adapters.md 3.3).
pub const R011: &str = "R011";

/// The `measured_from` fact a read reports for a dimension anchored at the sketch origin.
pub const SKETCH_ORIGIN: &str = "sketch origin";

fn warn(code: &str, message: String) -> AdapterDiagnostic {
    tracing::warn!(code, %message, "read fact finding");
    AdapterDiagnostic {
        code: code.to_owned(),
        severity: Severity::Warning,
        message,
    }
}

/// R007 (warning) for a bound dimension not measured from the sketch origin; other kinds never fire.
// frob:ticket 01M4Z8V9VNHK5QN6G0B0MC1ZA9
pub fn check_measured_from(kind: LocatorKind, measured_from: &str) -> Option<AdapterDiagnostic> {
    (kind == LocatorKind::Dimension && measured_from != SKETCH_ORIGIN).then(|| {
        warn(
            R007,
            format!("dimension is measured from `{measured_from}`, not the sketch origin"),
        )
    })
}

/// R011 (warning) when `instance_file` is not the slot's `primary` file.
// frob:ticket 01M4Z8V9VNHK5QN6G0B0MC1ZA9
pub fn check_component_file(
    instance: &str,
    primary: &str,
    instance_file: &str,
) -> Option<AdapterDiagnostic> {
    (instance_file != primary).then(|| {
        warn(
            R011,
            format!("instance `{instance}` uses `{instance_file}`, not the slot file `{primary}`"),
        )
    })
}

/// The instance files a slots read records, with R011 for each that is not `primary`.
// frob:ticket 01M4Z8V9VNHK5QN6G0B0MC1ZA9
pub fn observe_instance_files(
    primary: &str,
    instances: &BTreeMap<String, String>,
) -> (BTreeMap<String, String>, Vec<AdapterDiagnostic>) {
    let diagnostics = instances
        .iter()
        .filter_map(|(name, file)| check_component_file(name, primary, file))
        .collect();
    tracing::debug!(instances = instances.len(), "recorded instance files");
    (instances.clone(), diagnostics)
}

/// The lock's recorded values when the file's on-disk digest equals the lock's, so the read
/// can skip opening it in the tool; `None` means the file changed and must be opened.
// frob:ticket 01M4Z8V9VNHK5QN6G0B0MC1ZA9
pub fn reusable_values<'a>(
    on_disk_digest: &str,
    lock_digest: &str,
    lock_values: &'a BTreeMap<String, String>,
) -> Option<&'a BTreeMap<String, String>> {
    let same = on_disk_digest == lock_digest;
    tracing::debug!(same, "lock digest compared with the saved file");
    same.then_some(lock_values)
}

#[cfg(test)]
mod tests {
    use super::*;

    // frob:tests check_measured_from
    #[test]
    fn only_a_dimension_off_the_origin_is_r007() {
        assert_eq!(
            check_measured_from(LocatorKind::Dimension, "sketchedge").map(|d| d.code),
            Some(R007.to_owned())
        );
        assert!(check_measured_from(LocatorKind::Dimension, SKETCH_ORIGIN).is_none());
        assert!(check_measured_from(LocatorKind::Global, "sketchedge").is_none());
    }

    // frob:tests check_component_file
    // frob:tests observe_instance_files
    #[test]
    fn a_foreign_instance_file_is_r011_and_all_are_recorded() {
        let inst = BTreeMap::from([
            ("a<1>".to_owned(), "cad/x.SLDPRT".to_owned()),
            ("b<1>".to_owned(), "cad/y.SLDPRT".to_owned()),
        ]);
        let (seen, diags) = observe_instance_files("cad/x.SLDPRT", &inst);
        assert_eq!(seen, inst);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, R011);
        assert!(check_component_file("a<1>", "cad/x.SLDPRT", "cad/x.SLDPRT").is_none());
    }

    // frob:tests reusable_values
    #[test]
    fn equal_digests_reuse_the_lock_values() {
        let v = BTreeMap::from([("T".to_owned(), "10mm".to_owned())]);
        assert_eq!(reusable_values("blake3:aa", "blake3:aa", &v), Some(&v));
        assert_eq!(reusable_values("blake3:bb", "blake3:aa", &v), None);
    }
}
