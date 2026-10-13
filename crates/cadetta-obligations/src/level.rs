//! Definition versus instance level grouping of obligations.
//!
//! Specification: `docs/spec/obligations.md` section 2.1; `docs/spec/ir.md` section 2.
//! frob:ticket 01M4SWN74DYM9HNKRXYFY92Q27

use std::collections::BTreeMap;

use tracing::{debug, trace};

use crate::graph::{ObligationId, ObligationKind};

/// What a closure reads; decides the obligation level (obligations.md 2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClosureReads {
    /// The closure reads at least one component parameter.
    pub component_param: bool,
    /// The closure reads at least one interface-typed slot.
    pub interface_slot: bool,
}

/// Whether an obligation targets a definition path or instance paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Targets a definition path such as `acme.rover::Axle.seat`.
    Definition,
    /// Targets instance paths such as `acme.rover::Rover/axle[0].seat`.
    Instance,
}

impl ClosureReads {
    /// The level fixed by this closure: definition only if it reads neither kind of input.
    pub fn level(self) -> Level {
        if self.component_param || self.interface_slot {
            Level::Instance
        } else {
            Level::Definition
        }
    }
}

/// One instance path with the digest of its argument closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceClosure {
    /// Canonical instance path, e.g. `acme.rover::Rover/axle[0].seat`.
    pub path: String,
    /// Digest of the instance closure (ir.md 4).
    pub digest: String,
}

/// One grouped obligation: an id and every instance path it covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The obligation id (`<kind>:<target path>`).
    pub id: ObligationId,
    /// The level of the obligation.
    pub level: Level,
    /// Covered instance paths; empty at definition level.
    pub covered: Vec<String>,
}

/// Group a target into obligations: one at `definition_path` when `reads` is
/// definition level, else one per distinct argument-closure digest, targeting
/// the first instance path of each set of identical instances.
pub fn group(
    kind: ObligationKind,
    definition_path: &str,
    reads: ClosureReads,
    instances: &[InstanceClosure],
) -> Vec<Group> {
    if reads.level() == Level::Definition {
        debug!(%definition_path, "definition-level obligation");
        return vec![Group {
            id: ObligationId::new(kind, definition_path),
            level: Level::Definition,
            covered: Vec::new(),
        }];
    }
    // Preserve first-seen order of digests so output is deterministic.
    let mut order: Vec<&str> = Vec::new();
    let mut by_digest: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for inst in instances {
        trace!(path = %inst.path, digest = %inst.digest, "instance closure");
        let paths = by_digest.entry(inst.digest.as_str()).or_default();
        if paths.is_empty() {
            order.push(inst.digest.as_str());
        }
        paths.push(inst.path.clone());
    }
    debug!(%definition_path, groups = order.len(), "instance-level obligations");
    order
        .into_iter()
        .map(|d| {
            let covered = by_digest.remove(d).unwrap_or_default();
            Group {
                id: ObligationId::new(kind, covered[0].clone()),
                level: Level::Instance,
                covered,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inst(path: &str, digest: &str) -> InstanceClosure {
        InstanceClosure {
            path: path.into(),
            digest: digest.into(),
        }
    }

    // frob:tests crates/cadetta-obligations/src/level.rs::group
    #[test]
    fn no_component_or_slot_reads_is_definition_level() {
        let g = group(
            ObligationKind::Check,
            "acme.rover::Axle.seat",
            ClosureReads::default(),
            &[inst("acme.rover::Rover/axle[0].seat", "a")],
        );
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].level, Level::Definition);
        assert_eq!(g[0].id.path(), "acme.rover::Axle.seat");
        assert!(g[0].covered.is_empty());
    }

    #[test]
    fn identical_closures_form_one_obligation() {
        let reads = ClosureReads {
            component_param: true,
            interface_slot: false,
        };
        let insts = [
            inst("R/wheel[0]", "a"),
            inst("R/wheel[1]", "a"),
            inst("R/wheel[2]", "b"),
            inst("R/wheel[3]", "a"),
        ];
        let g = group(ObligationKind::Fit, "R.wheel", reads, &insts);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].level, Level::Instance);
        assert_eq!(g[0].covered, ["R/wheel[0]", "R/wheel[1]", "R/wheel[3]"]);
        assert_eq!(g[0].id.path(), "R/wheel[0]");
        assert_eq!(g[1].covered, ["R/wheel[2]"]);
    }

    #[test]
    fn interface_slot_read_is_instance_level() {
        let reads = ClosureReads {
            component_param: false,
            interface_slot: true,
        };
        assert_eq!(reads.level(), Level::Instance);
    }
}
