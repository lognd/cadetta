//! `cadetta lock tidy`: drop superseded duplicates and orphaned records, re-sort.
//!
//! Specification: `docs/spec/obligations.md` section 3 and `docs/spec/config-cli.md` section 3.
//! frob:ticket 01M4SWN7C44NFRF512T456X92X

use std::collections::HashSet;

use tracing::{debug, info};

use crate::graph::ObligationId;
use crate::lock::{Lock, LockError, Record};

/// What a tidy pass removed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TidyReport {
    /// Lines that lost the tie rule to another record with the same key.
    pub superseded: usize,
    /// Attestations whose obligation no longer exists.
    pub orphaned: Vec<ObligationId>,
}

impl TidyReport {
    /// Whether the pass changed nothing about the record set.
    pub fn is_clean(&self) -> bool {
        self.superseded == 0 && self.orphaned.is_empty()
    }
}

/// Drop `attest` records whose id is not in `live`; every other kind (notably `swap`) is kept.
pub fn drop_orphans(lock: &mut Lock, live: &HashSet<ObligationId>) -> Vec<ObligationId> {
    let dead: Vec<_> = lock
        .records()
        .filter_map(|r| match r {
            Record::Attest(a) if !live.contains(&a.id) => Some((r.key(), a.id.clone())),
            _ => None,
        })
        .collect();
    let mut gone = Vec::with_capacity(dead.len());
    for (key, id) in dead {
        lock.remove(&key);
        debug!(%id, "tidy dropped attestation of a removed obligation");
        gone.push(id);
    }
    gone
}

/// Tidy lock `text` against the `live` obligation set; returns the tidy lock and what was dropped.
/// Parsing applies the tie rule, so superseded duplicates vanish; rendering re-sorts.
pub fn tidy_text(
    text: &str,
    live: &HashSet<ObligationId>,
) -> Result<(Lock, TidyReport), LockError> {
    let mut lock = Lock::parse(text)?;
    let lines = text.lines().filter(|l| !l.is_empty()).count();
    let superseded = lines.saturating_sub(lock.len());
    let orphaned = drop_orphans(&mut lock, live);
    let report = TidyReport {
        superseded,
        orphaned,
    };
    info!(
        superseded = report.superseded,
        orphaned = report.orphaned.len(),
        kept = lock.len(),
        "lock tidied"
    );
    Ok((lock, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    const H1: &str = "aaaa";
    const H2: &str = "bbbb";

    fn attest(id: &str, at: &str, digest: &str) -> String {
        format!(
            "attest {id} role=r by=a@x.example at={at} digest={digest} verdict=holds margin=1mm deps= reason=\"ok\""
        )
    }

    fn live(ids: &[&str]) -> HashSet<ObligationId> {
        ids.iter()
            .map(|i| ObligationId::parse(i).unwrap())
            .collect()
    }

    // frob:tests crates/cadetta-obligations/src/lock_tidy.rs::tests.drops_superseded_and_orphans_only
    #[test]
    fn drops_superseded_and_orphans_only() {
        let text = format!(
            "{}\n{}\n{}\n{}\nswap S from=A to=B by=a@x.example at=2020-01-01 reason=\"why\"\n",
            attest("check:a", "2026-01-01", H1),
            attest("check:a", "2026-02-01", H2),
            attest("check:b", "2026-01-01", H1),
            attest("check:gone", "2026-01-01", H1),
        );
        let (lock, rep) = tidy_text(&text, &live(&["check:a", "check:b"])).unwrap();
        assert_eq!(rep.superseded, 1);
        assert_eq!(
            rep.orphaned,
            vec![ObligationId::parse("check:gone").unwrap()]
        );
        let a = lock
            .attestation(&ObligationId::parse("check:a").unwrap(), "r")
            .unwrap();
        assert_eq!(a.digest.as_str(), H2);
        assert!(
            lock.attestation(&ObligationId::parse("check:b").unwrap(), "r")
                .is_some()
        );
        assert!(lock.records().any(|r| matches!(r, Record::Swap(_))));
        assert_eq!(lock.len(), 3);
    }

    // frob:tests crates/cadetta-obligations/src/lock_tidy.rs::tests.live_lock_is_untouched_and_resorted
    #[test]
    fn live_lock_is_untouched_and_resorted() {
        let text = format!(
            "{}\n{}\n",
            attest("check:b", "2026-01-01", H1),
            attest("check:a", "2026-01-01", H1)
        );
        let (lock, rep) = tidy_text(&text, &live(&["check:a", "check:b"])).unwrap();
        assert!(rep.is_clean());
        assert_eq!(lock.len(), 2);
        assert!(Lock::is_canonical(&lock.render()).unwrap());
        assert_ne!(lock.render(), text);
    }
}
