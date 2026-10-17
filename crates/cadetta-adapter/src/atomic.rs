//! Snapshot and atomic restore around a per-document push (adapters.md 2.3).
//!
//! Before a push the host snapshots the current value of every locator it will
//! change in the document. If the rebuild fails (R004) the snapshot is written
//! back and the document rebuilt again, so no CAD file stays half-written. If
//! the restore itself fails, the diagnostic says so. After a push, only the
//! locators whose read-back matches the pushed value are eligible for a lock
//! `synced` record, so a partial sync reconverges on the next run.

use crate::sync::{PushItem, ReadBack, record_literal};
use crate::{AdapterDiagnostic, R004};

// frob:ticket 01M4SWMC85Q2YCHQ3GS4MREKBP
/// One document that can be read, written and rebuilt; the native side of a push.
pub trait Document {
    /// Reads the current value of each locator in `items` (the `value` field is ignored).
    fn read_current(&mut self, items: &[PushItem]) -> Vec<ReadBack>;

    /// Writes the values in `items` without rebuilding; `Err` carries the reason.
    fn write(&mut self, items: &[PushItem]) -> Result<(), String>;

    /// Rebuilds the document; `Err` carries the rebuild failure detail.
    fn rebuild(&mut self) -> Result<(), String>;
}

/// The pre-push values of the locators a push will change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    items: Vec<PushItem>,
}

impl Snapshot {
    /// Captures the current values of the locators in `items` from `doc`.
    pub fn take(doc: &mut dyn Document, items: &[PushItem]) -> Self {
        let back = doc.read_current(items);
        let items: Vec<PushItem> = items
            .iter()
            .filter_map(|it| {
                let b = back.iter().find(|b| b.locator == it.locator)?;
                let value = b.value.clone()?;
                Some(PushItem {
                    path: it.path.clone(),
                    locator: it.locator.clone(),
                    value,
                    tolerance: b.tolerance.clone(),
                    kind: it.kind,
                })
            })
            .collect();
        tracing::debug!(
            captured = items.len(),
            wanted = back.len(),
            "snapshot taken"
        );
        Self { items }
    }

    /// The captured locators, with their old values as push items.
    pub fn items(&self) -> &[PushItem] {
        &self.items
    }

    /// Writes the old values back and rebuilds; `Err` explains a failed restore.
    pub fn restore(&self, doc: &mut dyn Document) -> Result<(), String> {
        tracing::info!(locators = self.items.len(), "restoring pre-push snapshot");
        doc.write(&self.items)
            .map_err(|e| format!("could not set the old values: {e}"))?;
        doc.rebuild()
            .map_err(|e| format!("could not rebuild after restore: {e}"))
    }
}

/// What an atomic push did to one document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AtomicOutcome {
    /// Read-back per locator; all values `None` after a failed push.
    pub read_back: Vec<ReadBack>,
    /// R004 diagnostics, including a failed-restore note.
    pub diagnostics: Vec<AdapterDiagnostic>,
    /// True when the snapshot was written back after a failure.
    pub restored: bool,
}

/// Pushes `items` into `doc`, rebuilds, and restores the snapshot on failure.
///
/// With `regen` false the values are written but not rebuilt, so there is no
/// R004 to recover from.
pub fn push_atomic(
    doc: &mut dyn Document,
    target: &str,
    items: &[PushItem],
    regen: bool,
) -> AtomicOutcome {
    let snap = Snapshot::take(doc, items);
    let failure = match doc.write(items) {
        Err(e) => Some(format!("write failed: {e}")),
        Ok(()) if regen => doc.rebuild().err(),
        Ok(()) => None,
    };
    let Some(detail) = failure else {
        tracing::debug!(target, "push ok");
        return AtomicOutcome {
            read_back: doc.read_current(items),
            ..AtomicOutcome::default()
        };
    };
    let mut diagnostics = vec![AdapterDiagnostic::rebuild_failure(target, &detail)];
    let restored = match snap.restore(doc) {
        Ok(()) => true,
        Err(why) => {
            tracing::error!(target, %why, "restore failed; document may be half-pushed");
            diagnostics.push(AdapterDiagnostic {
                code: R004.to_string(),
                severity: crate::Severity::Error,
                message: format!("restore of {target} failed, document may be half-pushed: {why}"),
            });
            false
        }
    };
    let read_back = items
        .iter()
        .map(|it| ReadBack {
            locator: it.locator.clone(),
            value: None,
            tolerance: None,
        })
        .collect();
    AtomicOutcome {
        read_back,
        diagnostics,
        restored,
    }
}

/// Locators whose read-back equals the pushed record; only these get a `synced` record.
pub fn matched_locators(items: &[PushItem], back: &[ReadBack]) -> Vec<String> {
    items
        .iter()
        .filter(|it| {
            let want = record_literal(&it.value, it.tolerance.as_ref());
            back.iter().any(|b| {
                b.locator == it.locator
                    && b.value
                        .as_deref()
                        .map(|v| record_literal(v, b.tolerance.as_ref()))
                        .as_deref()
                        == Some(want.as_str())
            })
        })
        .map(|it| it.locator.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Fake {
        vals: BTreeMap<String, String>,
        fail_rebuild_when: Option<String>,
        fail_restore_rebuild: bool,
        drop_locator: Option<String>,
        rebuilds: usize,
    }

    impl Document for Fake {
        fn read_current(&mut self, items: &[PushItem]) -> Vec<ReadBack> {
            items
                .iter()
                .map(|i| ReadBack {
                    locator: i.locator.clone(),
                    value: if self.drop_locator.as_deref() == Some(&i.locator) {
                        Some("999".into())
                    } else {
                        self.vals.get(&i.locator).cloned()
                    },
                    tolerance: None,
                })
                .collect()
        }
        fn write(&mut self, items: &[PushItem]) -> Result<(), String> {
            for i in items {
                self.vals.insert(i.locator.clone(), i.value.clone());
            }
            Ok(())
        }
        fn rebuild(&mut self) -> Result<(), String> {
            self.rebuilds += 1;
            if let Some(bad) = &self.fail_rebuild_when
                && self.vals.values().any(|v| v == bad)
            {
                return Err("bad feature".into());
            }
            if self.fail_restore_rebuild && self.rebuilds > 1 {
                return Err("tool gone".into());
            }
            Ok(())
        }
    }

    fn item(loc: &str, v: &str) -> PushItem {
        PushItem {
            path: format!("p.{loc}"),
            locator: loc.into(),
            value: v.into(),
            tolerance: None,
            kind: None,
        }
    }

    fn doc() -> Fake {
        let mut d = Fake::default();
        d.vals.insert("a".into(), "1".into());
        d.vals.insert("b".into(), "2".into());
        d
    }

    // frob:tests push_atomic
    #[test]
    fn regen_failure_restores_snapshot() {
        let mut d = doc();
        d.fail_rebuild_when = Some("50".into());
        let out = push_atomic(&mut d, "Part1", &[item("a", "50"), item("b", "3")], true);
        assert!(out.restored);
        assert_eq!(out.diagnostics[0].code, "R004");
        assert_eq!(d.vals["a"], "1");
        assert_eq!(d.vals["b"], "2");
        assert!(out.read_back.iter().all(|b| b.value.is_none()));
    }

    // frob:tests push_atomic
    #[test]
    fn failed_restore_is_reported() {
        let mut d = doc();
        d.fail_rebuild_when = Some("50".into());
        d.fail_restore_rebuild = true;
        let out = push_atomic(&mut d, "Part1", &[item("a", "50")], true);
        assert!(!out.restored);
        assert_eq!(out.diagnostics.len(), 2);
        assert!(out.diagnostics[1].message.contains("half-pushed"));
    }

    // frob:tests push_atomic
    #[test]
    fn successful_push_keeps_new_values() {
        let mut d = doc();
        let out = push_atomic(&mut d, "Part1", &[item("a", "7")], true);
        assert!(out.diagnostics.is_empty() && !out.restored);
        assert_eq!(d.vals["a"], "7");
        assert_eq!(out.read_back[0].value.as_deref(), Some("7"));
    }

    // frob:tests matched_locators
    #[test]
    fn only_matching_readback_is_recorded() {
        let mut d = doc();
        d.drop_locator = Some("b".into());
        let items = [item("a", "7"), item("b", "8")];
        let out = push_atomic(&mut d, "Part1", &items, true);
        assert_eq!(matched_locators(&items, &out.read_back), vec!["a"]);
    }
}
