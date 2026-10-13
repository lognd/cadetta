//! `cadetta lock merge BASE OURS THEIRS`: the git merge driver for `cadetta.lock`.
//!
//! Specification: `docs/spec/obligations.md` section 3 (Merge driver, Tie rule).
//! frob:ticket 01M4SWN78DB7D7SPY3X8BHQT8Y

use tracing::{debug, info, warn};

use crate::lock::{Lock, LockError, Record, RecordKey};

/// Two sides disagreed about one `synced` key; neither is chosen silently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    /// The contested record key.
    pub key: RecordKey,
    /// Our rendered line.
    pub ours: String,
    /// Their rendered line.
    pub theirs: String,
}

/// The result of a merge: the merged lock and any unresolved conflicts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merged {
    /// Union of both sides, tie rule applied, conflicted keys left out.
    pub lock: Lock,
    /// Keys left for a human (differing `synced` values).
    pub conflicts: Vec<Conflict>,
    /// Number of records dropped because `valid` rejected them.
    pub dropped_invalid: usize,
}

impl Merged {
    /// The file bytes: canonical lock text, then git conflict blocks for each conflict.
    pub fn text(&self) -> String {
        let mut s = self.lock.render();
        for c in &self.conflicts {
            s.push_str("<<<<<<< ours\n");
            s.push_str(&c.ours);
            s.push_str("\n=======\n");
            s.push_str(&c.theirs);
            s.push_str("\n>>>>>>> theirs\n");
        }
        s
    }

    /// Whether the merge is clean (git driver exit status 0).
    pub fn is_clean(&self) -> bool {
        self.conflicts.is_empty()
    }
}

fn is_synced(r: &Record) -> bool {
    matches!(r, Record::SyncedValue(_) | Record::SyncedFile(_))
}

/// Merge three lock texts, accepting every record (no IR available to validate digests).
pub fn merge_text(base: &str, ours: &str, theirs: &str) -> Result<Merged, LockError> {
    merge_text_with(base, ours, theirs, |_| true)
}

/// Merge three lock texts, keeping only records for which `valid` holds.
pub fn merge_text_with(
    base: &str,
    ours: &str,
    theirs: &str,
    valid: impl Fn(&Record) -> bool,
) -> Result<Merged, LockError> {
    let base = Lock::parse(base)?;
    let ours = Lock::parse(ours)?;
    let theirs = Lock::parse(theirs)?;
    Ok(merge(&base, &ours, &theirs, valid))
}

/// Union `ours` and `theirs` by key with the tie rule; `synced` keys changed differently on
/// both sides (relative to `base`) become conflicts. Records failing `valid` are dropped.
pub fn merge(base: &Lock, ours: &Lock, theirs: &Lock, valid: impl Fn(&Record) -> bool) -> Merged {
    let mut lock = Lock::new();
    let mut conflicts = Vec::new();
    let mut dropped_invalid = 0;
    let mut keys: Vec<RecordKey> = ours
        .records()
        .chain(theirs.records())
        .map(Record::key)
        .collect();
    keys.sort();
    keys.dedup();
    for key in keys {
        let (o, t) = (ours.get(&key), theirs.get(&key));
        let chosen: Option<&Record> = match (o, t) {
            (Some(o), Some(t)) if o == t => Some(o),
            (Some(o), Some(t)) if is_synced(o) => {
                let b = base.get(&key);
                if b == Some(o) {
                    Some(t)
                } else if b == Some(t) {
                    Some(o)
                } else {
                    warn!(kind = key.0, key = ?key.1, "synced value conflict");
                    conflicts.push(Conflict {
                        key: key.clone(),
                        ours: o.render(),
                        theirs: t.render(),
                    });
                    None
                }
            }
            (Some(o), Some(t)) => {
                let ord = o
                    .at()
                    .cmp(&t.at())
                    .then_with(|| o.render().cmp(&t.render()));
                Some(if ord.is_ge() { o } else { t })
            }
            (Some(r), None) | (None, Some(r)) => Some(r),
            (None, None) => None,
        };
        if let Some(r) = chosen {
            if valid(r) {
                lock.insert(r.clone());
            } else {
                debug!(kind = key.0, key = ?key.1, "invalid record dropped by merge");
                dropped_invalid += 1;
            }
        }
    }
    info!(
        records = lock.len(),
        conflicts = conflicts.len(),
        dropped_invalid,
        "lock merged"
    );
    Merged {
        lock,
        conflicts,
        dropped_invalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "attest fit:acme::a role=r by=x at=2026-10-01 digest=ab verdict=holds margin=1 deps= reason=\"\"\n";
    const A_NEW: &str = "attest fit:acme::a role=r by=y at=2026-10-02 digest=cd verdict=holds margin=1 deps= reason=\"\"\n";
    const B: &str = "attest fit:acme::b role=r by=x at=2026-10-01 digest=ab verdict=holds margin=1 deps= reason=\"\"\n";
    const S1: &str = "synced freecad acme::d locator=L value=1\n";
    const S2: &str = "synced freecad acme::d locator=L value=2\n";

    // frob:tests crates/cadetta-obligations/src/lock_merge.rs::merge
    #[test]
    fn unions_in_sorted_order_with_tie_rule() {
        let ours = format!("{B}{A}");
        let theirs = A_NEW.to_string();
        let m = merge_text("", &ours, &theirs).unwrap();
        assert!(m.is_clean());
        assert_eq!(m.text(), format!("{A_NEW}{B}"));
        let flipped = merge_text("", &theirs, &ours).unwrap();
        assert_eq!(flipped.text(), m.text());
    }

    // frob:tests crates/cadetta-obligations/src/lock_merge.rs::merge
    #[test]
    fn differing_synced_values_conflict() {
        let m = merge_text(S1, S2, "synced freecad acme::d locator=L value=3\n").unwrap();
        assert_eq!(m.conflicts.len(), 1);
        assert!(m.text().contains("<<<<<<< ours"));
    }

    // frob:tests crates/cadetta-obligations/src/lock_merge.rs::merge
    #[test]
    fn one_sided_synced_change_wins() {
        let m = merge_text(S1, S1, S2).unwrap();
        assert!(m.is_clean());
        assert_eq!(m.text(), S2);
    }

    // frob:tests crates/cadetta-obligations/src/lock_merge.rs::merge
    #[test]
    fn invalid_records_are_dropped() {
        let m = merge_text_with("", A, B, |r| r.render() != A.trim_end()).unwrap();
        assert_eq!(m.dropped_invalid, 1);
        assert_eq!(m.text(), B);
    }
}
