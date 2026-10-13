//! Sync orchestration (adapters.md 2, 2.1; config-cli.md 3.1): scope the
//! locators, classify each three-way, push through a [`Backend`], read back and
//! compute the lock updates. Pure apart from the backend calls.
//!
//! `--dry-run` calls no backend and changes no lock; `--no-regen` pushes but
//! leaves the lock untouched; a run where every locator is no-op or converged
//! pushes nothing and leaves the lock identical (idempotent).

use std::collections::BTreeMap;
use std::fmt;

use crate::classify::{Action, Class, R001, classify};
use crate::rpc::DimTolerance;
use crate::scope::Scope;

/// Diagnostic code for a push that replaces a native expression (adapters.md 2.1).
pub const R006: &str = "R006";

/// Diagnostic code for a locator whose native item is gone since a sync recorded it (adapters.md 3.5).
// frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
pub const R003: &str = "R003";

/// R003 for each implicit locator `(path, locator)` with a lock base that the read no longer finds.
///
/// `found` holds the `(path, locator)` pairs the read still sees; an implicit locator never
/// recorded by a sync is not reported (nothing to lose).
// frob:ticket 01M4VB640810PZEZHQJ1T5WFTK
pub fn missing_implicit(
    adapter: &str,
    implicit: &[(String, String)],
    found: &std::collections::BTreeSet<(String, String)>,
    lock: &Lock,
) -> Vec<Diag> {
    implicit
        .iter()
        .filter(|(p, l)| !found.contains(&(p.clone(), l.clone())))
        .filter(|(p, l)| lock.contains_key(&(adapter.to_owned(), p.clone(), l.clone())))
        .map(|(p, l)| {
            tracing::warn!(adapter, path = %p, locator = %l, "implicit locator vanished");
            Diag {
                code: R003,
                message: format!("{p}: native item `{l}` recorded by a sync is gone"),
            }
        })
        .collect()
}

/// One bound locator with its three-way inputs, as emitted literals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Adapter instance id.
    pub adapter: String,
    /// Native document the locator lives in.
    pub file: String,
    /// Canonical parameter path.
    pub path: String,
    /// Native locator.
    pub locator: String,
    /// Lock `synced` value, if any.
    pub base: Option<String>,
    /// Freshly computed value.
    pub ours: String,
    /// Value read from the document before any push.
    pub theirs: String,
    /// Native expression a push would replace (R006), if any.
    pub replaces: Option<String>,
    /// Design tolerance to write with `ours` (adapters.md 3.6); `None` writes none.
    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    pub ours_tol: Option<DimTolerance>,
    /// Tolerance read from the document before any push.
    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    pub theirs_tol: Option<DimTolerance>,
}

/// The compared literal of a value and its tolerance: `12mm`, or `12mm +-0.1mm`.
///
/// `base`, `ours` and `theirs` are classified, and the lock records `synced`, in this form, so a
/// tolerance edited in CAD is R001 like an edited value (adapters.md 3.6).
// frob:ticket 01M4VB648PMGRPG6R8D606HZK6
pub fn record_literal(value: &str, tol: Option<&DimTolerance>) -> String {
    match tol {
        Some(t) => format!("{value} {t}"),
        None => value.to_owned(),
    }
}

impl Entry {
    /// `ours` with its tolerance, the literal pushed and recorded.
    pub fn ours_record(&self) -> String {
        record_literal(&self.ours, self.ours_tol.as_ref())
    }

    /// `theirs` with its tolerance.
    pub fn theirs_record(&self) -> String {
        record_literal(&self.theirs, self.theirs_tol.as_ref())
    }
}

/// Flags of one `sync` run (config-cli.md 3.1).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// Print what would happen; write nothing.
    pub dry_run: bool,
    /// Canonical-path globs; empty means all.
    pub only: Vec<String>,
    /// Push without rebuilding; the lock is not updated.
    pub no_regen: bool,
    /// Push over any R001.
    pub force_design: bool,
    /// Adapter ids to run; empty means all.
    pub adapters: Vec<String>,
}

/// Per-locator verb of the sync output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    /// Pushed (or would push, in a dry run).
    Push,
    /// Nothing to do.
    Skip,
    /// Refused: the CAD moved (R001).
    Drift,
    /// Recorded without a push.
    Converged,
    /// Pushed but the read-back did not match.
    Fail,
}

impl fmt::Display for Verb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Push => "push",
            Self::Skip => "skip",
            Self::Drift => "drift",
            Self::Converged => "converged",
            Self::Fail => "fail",
        })
    }
}

/// One output record: the classified locator and its outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The locator's inputs.
    pub entry: Entry,
    /// Three-way class.
    pub class: Class,
    /// Verb for the output line.
    pub verb: Verb,
}

impl Record {
    /// Renders `verb  path  locator  old -> new`.
    pub fn line(&self) -> String {
        let theirs = self.entry.theirs_record();
        let old = self.entry.base.as_deref().unwrap_or(&theirs);
        format!(
            "{}  {}  {}  {} -> {}",
            self.verb,
            self.entry.path,
            self.entry.locator,
            old,
            self.entry.ours_record()
        )
    }
}

/// A diagnostic raised by the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    /// Rule code (`R001`, `R006`).
    pub code: &'static str,
    /// Message.
    pub message: String,
}

/// A lock `synced` key: adapter id, canonical path, locator.
pub type LockKey = (String, String, String);

/// The `synced` records of the lock the run reads and updates.
pub type Lock = BTreeMap<LockKey, String>;

/// One locator pushed: path, locator, value and the tolerance written with it.
// frob:ticket 01M4VB648PMGRPG6R8D606HZK6
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushItem {
    /// Canonical parameter path.
    pub path: String,
    /// Native locator.
    pub locator: String,
    /// The value literal.
    pub value: String,
    /// The tolerance to write, if any.
    pub tolerance: Option<DimTolerance>,
}

/// The read-back of one pushed locator.
// frob:ticket 01M4VB648PMGRPG6R8D606HZK6
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadBack {
    /// Native locator.
    pub locator: String,
    /// Value read back; `None` when the locator failed or was rejected.
    pub value: Option<String>,
    /// Tolerance read back.
    pub tolerance: Option<DimTolerance>,
}

/// The adapter side of a push: write, optionally rebuild, and read back.
pub trait Backend {
    /// Pushes `items` into `file`; returns the read-back per locator.
    fn push(&mut self, adapter: &str, file: &str, items: &[PushItem], regen: bool)
    -> Vec<ReadBack>;
}

/// Everything a run produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// One record per in-scope locator, in input order.
    pub records: Vec<Record>,
    /// R001 and R006 diagnostics.
    pub diagnostics: Vec<Diag>,
    /// Whether the lock was changed.
    pub lock_written: bool,
}

impl Report {
    /// True when any locator was refused with R001 (exit 1).
    pub fn has_r001(&self) -> bool {
        self.records
            .iter()
            .any(|r| r.class.is_r001() && r.verb == Verb::Drift)
    }

    /// Output lines, one per locator.
    pub fn lines(&self) -> Vec<String> {
        self.records.iter().map(Record::line).collect()
    }
}

fn lock_key(e: &Entry) -> LockKey {
    (e.adapter.clone(), e.path.clone(), e.locator.clone())
}

/// Runs the loop over `entries`; updates `lock` unless dry-run or no-regen.
pub fn run(
    entries: &[Entry],
    opts: &Options,
    backend: &mut dyn Backend,
    lock: &mut Lock,
) -> Report {
    let scope = Scope::new(vec![], opts.only.clone());
    let mut report = Report::default();
    for e in entries {
        if !opts.adapters.is_empty() && !opts.adapters.contains(&e.adapter) {
            tracing::debug!(adapter = %e.adapter, "adapter not selected");
            continue;
        }
        if !scope.accepts(&e.path) {
            tracing::debug!(path = %e.path, "path outside --only");
            continue;
        }
        let class = classify(e.base.as_deref(), &e.ours_record(), &e.theirs_record());
        let action = class.action(opts.force_design);
        let verb = match action {
            Action::Nothing => Verb::Skip,
            Action::Push => Verb::Push,
            Action::RecordOnly => Verb::Converged,
            Action::Refuse => Verb::Drift,
        };
        if let Some(msg) = class.r001_message()
            && (action == Action::Refuse || opts.force_design)
        {
            report.diagnostics.push(Diag {
                code: R001,
                message: format!(
                    "{msg}: {} base={:?} ours={} theirs={}",
                    e.path,
                    e.base,
                    e.ours_record(),
                    e.theirs_record()
                ),
            });
        }
        if verb == Verb::Push
            && let Some(old) = &e.replaces
        {
            report.diagnostics.push(Diag {
                code: R006,
                message: format!("{} replaces native expression `{old}`", e.locator),
            });
        }
        report.records.push(Record {
            entry: e.clone(),
            class,
            verb,
        });
    }
    if opts.dry_run {
        tracing::info!(records = report.records.len(), "dry run: nothing written");
        return report;
    }
    execute(&mut report, opts, backend, lock);
    report
}

/// Pushes the `Push` records per (adapter, file) and updates the lock.
fn execute(report: &mut Report, opts: &Options, backend: &mut dyn Backend, lock: &mut Lock) {
    let mut groups: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for (i, r) in report.records.iter().enumerate() {
        if r.verb == Verb::Push {
            groups
                .entry((r.entry.adapter.clone(), r.entry.file.clone()))
                .or_default()
                .push(i);
        }
    }
    let mut updates: Vec<(LockKey, String)> = Vec::new();
    for ((adapter, file), idx) in groups {
        let items: Vec<PushItem> = idx
            .iter()
            .map(|&i| {
                let e = &report.records[i].entry;
                PushItem {
                    path: e.path.clone(),
                    locator: e.locator.clone(),
                    value: e.ours.clone(),
                    tolerance: e.ours_tol.clone(),
                }
            })
            .collect();
        let back = backend.push(&adapter, &file, &items, !opts.no_regen);
        for &i in &idx {
            let rec = &mut report.records[i];
            let got = back
                .iter()
                .find(|b| b.locator == rec.entry.locator)
                .and_then(|b| {
                    b.value
                        .as_deref()
                        .map(|v| record_literal(v, b.tolerance.as_ref()))
                });
            if got.as_deref() == Some(rec.entry.ours_record().as_str()) {
                updates.push((lock_key(&rec.entry), rec.entry.ours_record()));
            } else {
                tracing::warn!(locator = %rec.entry.locator, "read-back mismatch");
                rec.verb = Verb::Fail;
            }
        }
    }
    if opts.no_regen {
        tracing::info!("no-regen: lock not updated");
        return;
    }
    for r in &report.records {
        if r.verb == Verb::Converged {
            updates.push((lock_key(&r.entry), r.entry.ours_record()));
        }
    }
    for (k, v) in updates {
        if lock.get(&k) != Some(&v) {
            lock.insert(k, v);
            report.lock_written = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Mock {
        calls: usize,
        regen: Vec<bool>,
        fail: bool,
        drop_tol: bool,
    }

    impl Backend for Mock {
        fn push(&mut self, _a: &str, _f: &str, items: &[PushItem], regen: bool) -> Vec<ReadBack> {
            self.calls += 1;
            self.regen.push(regen);
            items
                .iter()
                .map(|i| ReadBack {
                    locator: i.locator.clone(),
                    value: (!self.fail).then(|| i.value.clone()),
                    tolerance: if self.drop_tol {
                        None
                    } else {
                        i.tolerance.clone()
                    },
                })
                .collect()
        }
    }

    fn entry(path: &str, base: Option<&str>, ours: &str, theirs: &str) -> Entry {
        Entry {
            adapter: "fc".into(),
            file: "a.FCStd".into(),
            path: path.into(),
            locator: format!("L_{path}"),
            base: base.map(str::to_owned),
            ours: ours.into(),
            theirs: theirs.into(),
            replaces: None,
            ours_tol: None,
            theirs_tol: None,
        }
    }

    /// Entries derived from the current lock and a CAD that holds `cad`.
    fn entries(lock: &Lock, cad: &BTreeMap<String, String>, design: &[(&str, &str)]) -> Vec<Entry> {
        design
            .iter()
            .map(|(p, v)| {
                let mut e = entry(p, None, v, cad.get(*p).map(String::as_str).unwrap_or("0"));
                e.base = lock.get(&lock_key(&e)).cloned();
                e
            })
            .collect()
    }

    #[test]
    fn second_run_pushes_nothing_and_lock_identical() {
        let design = [("p.x", "2"), ("p.y", "5")];
        let mut cad: BTreeMap<String, String> = BTreeMap::new();
        cad.insert("p.y".into(), "5".into());
        let mut lock = Lock::new();
        let mut mock = Mock::default();
        let es = entries(&lock, &cad, &design);
        let r1 = run(&es, &Options::default(), &mut mock, &mut lock);
        assert_eq!(mock.calls, 1);
        assert!(r1.lock_written);
        cad.insert("p.x".into(), "2".into());
        let snapshot = lock.clone();
        let es = entries(&lock, &cad, &design);
        let r2 = run(&es, &Options::default(), &mut mock, &mut lock);
        assert_eq!(mock.calls, 1, "second run pushed");
        assert!(!r2.lock_written);
        assert_eq!(lock, snapshot);
        assert!(r2.records.iter().all(|r| r.verb == Verb::Skip));
    }

    #[test]
    fn dry_run_writes_nothing_and_lists_r006() {
        let mut e = entry("p.x", Some("1"), "2", "1");
        e.replaces = Some("=\"A\"*2".into());
        let mut lock = Lock::new();
        let mut mock = Mock::default();
        let r = run(
            &[e],
            &Options {
                dry_run: true,
                ..Default::default()
            },
            &mut mock,
            &mut lock,
        );
        assert_eq!(mock.calls, 0);
        assert!(lock.is_empty());
        assert_eq!(r.records[0].verb, Verb::Push);
        assert_eq!(r.diagnostics.len(), 1);
        assert_eq!(r.diagnostics[0].code, R006);
        assert!(r.diagnostics[0].message.contains("\"A\"*2"));
        assert_eq!(r.lines()[0], "push  p.x  L_p.x  1 -> 2");
    }

    #[test]
    fn only_and_adapter_id_scope() {
        let mut other = entry("q.z", Some("1"), "2", "1");
        other.adapter = "sw".into();
        let es = vec![
            entry("p.x", Some("1"), "2", "1"),
            entry("p.y", Some("1"), "2", "1"),
            other,
        ];
        let mut lock = Lock::new();
        let mut mock = Mock::default();
        let o = Options {
            only: vec!["p.x".into()],
            ..Default::default()
        };
        let r = run(&es, &o, &mut mock, &mut lock);
        assert_eq!(r.records.len(), 1);
        let o = Options {
            adapters: vec!["sw".into()],
            ..Default::default()
        };
        let r = run(&es, &o, &mut mock, &mut lock);
        assert_eq!(r.records.len(), 1);
        assert_eq!(r.records[0].entry.adapter, "sw");
    }

    #[test]
    fn no_regen_pushes_without_lock_update() {
        let es = vec![entry("p.x", Some("1"), "2", "1")];
        let mut lock = Lock::new();
        let mut mock = Mock::default();
        let o = Options {
            no_regen: true,
            ..Default::default()
        };
        let r = run(&es, &o, &mut mock, &mut lock);
        assert_eq!(mock.calls, 1);
        assert_eq!(mock.regen, vec![false]);
        assert!(lock.is_empty());
        assert!(!r.lock_written);
    }

    #[test]
    fn drift_refused_and_force_design_pushes() {
        let es = vec![entry("p.x", Some("1"), "1", "3")];
        let mut lock = Lock::new();
        let mut mock = Mock::default();
        let r = run(&es, &Options::default(), &mut mock, &mut lock);
        assert!(r.has_r001());
        assert_eq!(mock.calls, 0);
        let o = Options {
            force_design: true,
            ..Default::default()
        };
        let r = run(&es, &o, &mut mock, &mut lock);
        assert!(!r.has_r001());
        assert_eq!(mock.calls, 1);
    }

    #[test]
    fn readback_mismatch_fails_and_skips_lock() {
        let es = vec![entry("p.x", Some("1"), "2", "1")];
        let mut lock = Lock::new();
        let mut mock = Mock {
            fail: true,
            ..Default::default()
        };
        let r = run(&es, &Options::default(), &mut mock, &mut lock);
        assert_eq!(r.records[0].verb, Verb::Fail);
        assert!(lock.is_empty());
    }

    #[test]
    fn converged_recorded_in_lock() {
        let es = vec![entry("p.x", Some("1"), "2", "2")];
        let mut lock = Lock::new();
        let mut mock = Mock::default();
        let r = run(&es, &Options::default(), &mut mock, &mut lock);
        assert_eq!(r.records[0].verb, Verb::Converged);
        assert_eq!(mock.calls, 0);
        assert_eq!(lock.len(), 1);
    }

    fn tol(t: &str) -> Option<DimTolerance> {
        Some(DimTolerance::Symmetric { t: t.into() })
    }

    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    // frob:tests tolerance_edit_in_cad_is_r001
    #[test]
    fn tolerance_edit_in_cad_is_r001() {
        let mut e = entry("p.x", Some("12mm +-0.1mm"), "12mm", "12mm");
        e.ours_tol = tol("0.1mm");
        e.theirs_tol = tol("0.3mm");
        let mut lock = Lock::new();
        let mut mock = Mock::default();
        let r = run(&[e.clone()], &Options::default(), &mut mock, &mut lock);
        assert_eq!(r.records[0].class, Class::CadMoved);
        assert_eq!(r.records[0].verb, Verb::Drift);
        assert!(r.has_r001());
        assert_eq!(r.diagnostics[0].code, R001);
        assert!(r.diagnostics[0].message.contains("+-0.3mm"));
        assert_eq!(mock.calls, 0);
        // the same tolerance read back is not drift
        e.theirs_tol = tol("0.1mm");
        let r = run(&[e], &Options::default(), &mut mock, &mut lock);
        assert_eq!(r.records[0].verb, Verb::Skip);
        assert!(!r.has_r001());
    }

    // frob:ticket 01M4VB648PMGRPG6R8D606HZK6
    // frob:tests synced_record_includes_tolerance
    #[test]
    fn synced_record_includes_tolerance() {
        let mut e = entry("p.x", None, "12mm", "11mm");
        e.ours_tol = tol("0.1mm");
        let key = lock_key(&e);
        let mut lock = Lock::new();
        let mut mock = Mock::default();
        let r = run(&[e.clone()], &Options::default(), &mut mock, &mut lock);
        assert_eq!(r.records[0].verb, Verb::Push);
        assert_eq!(lock[&key], "12mm +-0.1mm");
        // a tool that drops the tolerance fails the read-back compare
        let mut lock = Lock::new();
        let mut mock = Mock {
            drop_tol: true,
            ..Default::default()
        };
        let r = run(&[e], &Options::default(), &mut mock, &mut lock);
        assert_eq!(r.records[0].verb, Verb::Fail);
        assert!(lock.is_empty());
    }

    // frob:tests implicit_vanished_after_sync_is_r003
    #[test]
    fn implicit_vanished_after_sync_is_r003() {
        let implicit = vec![
            ("a::B.x".to_owned(), "G1".to_owned()),
            ("a::B.y".to_owned(), "G2".to_owned()),
        ];
        let mut lock = Lock::new();
        lock.insert(("sw".into(), "a::B.x".into(), "G1".into()), "5".into());
        let found = std::collections::BTreeSet::new();
        let d = missing_implicit("sw", &implicit, &found, &lock);
        assert_eq!(d.len(), 1, "only the recorded one is lost");
        assert_eq!(d[0].code, R003);
        let mut found = found;
        found.insert(("a::B.x".to_owned(), "G1".to_owned()));
        assert!(missing_implicit("sw", &implicit, &found, &lock).is_empty());
    }
}
