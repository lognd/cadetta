//! Upstream attestations and trust roles (packages.md 6).
//!
//! A package's lock may carry attestations of its own nodes and they travel with the package:
//! a dependent sees them as "attested upstream". `[dependencies."acme.shop"] trust = [...]`
//! lets the named roles discharge the dependent's matching obligations on the dependency's
//! nodes; without `trust` they are information only.
//! frob:ticket 01M4SXS3VRYKPBZJ391SQRBG5C

use std::collections::{BTreeMap, BTreeSet};

use tracing::{debug, trace};

use crate::graph::ObligationId;
use crate::lock::{Attest, Lock, Record};

/// One attestation a dependency's lock carries for one of its own nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamAttest {
    /// The dependency package the attestation travelled with.
    pub package: String,
    /// The attestation as recorded upstream.
    pub attest: Attest,
}

impl UpstreamAttest {
    /// The "attested upstream" description shown by `cadetta obligations`.
    pub fn describe(&self) -> String {
        format!(
            "{} {} by {} on {}",
            self.package,
            self.attest.role,
            self.attest.by,
            self.attest.at.as_str()
        )
    }
}

/// Every upstream attestation of the dependencies in scope, in load order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Upstream {
    items: Vec<UpstreamAttest>,
}

impl Upstream {
    /// No upstream attestations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Take the attestations of `lock`, the lock of dependency `package`, along with it.
    pub fn add_package(&mut self, package: &str, lock: &Lock) {
        let before = self.items.len();
        for rec in lock.records() {
            if let Record::Attest(a) = rec {
                self.items.push(UpstreamAttest {
                    package: package.to_owned(),
                    attest: a.clone(),
                });
            }
        }
        debug!(
            package,
            attestations = self.items.len() - before,
            "upstream attestations loaded"
        );
    }

    /// Whether no dependency carries an attestation.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The upstream attestations of the obligation `id`.
    pub fn for_id<'a>(&'a self, id: &'a ObligationId) -> impl Iterator<Item = &'a UpstreamAttest> {
        self.items.iter().filter(move |u| &u.attest.id == id)
    }
}

/// The roles each dependency is trusted for (`trust` under `[dependencies.NAME]`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Trust {
    roles: BTreeMap<String, BTreeSet<String>>,
}

impl Trust {
    /// Trust nothing: upstream attestations are information only.
    pub fn new() -> Self {
        Self::default()
    }

    /// Trust `package`'s upstream attestations made in `roles`; an empty list trusts nothing.
    pub fn set(&mut self, package: &str, roles: &[String]) {
        if roles.is_empty() {
            return;
        }
        debug!(package, ?roles, "upstream roles trusted");
        self.roles
            .entry(package.to_owned())
            .or_default()
            .extend(roles.iter().cloned());
    }

    /// Whether `package`'s upstream attestations in `role` are trusted.
    pub fn trusts(&self, package: &str, role: &str) -> bool {
        self.roles.get(package).is_some_and(|r| r.contains(role))
    }

    /// The upstream attestation that discharges `id`, if any.
    ///
    /// It must come from a trusted package and role, the role must be one of the dependent's
    /// `covering` roles for `id`, and the attested closure digest must equal the `current` one
    /// so a changed node is never discharged by an old sign-off.
    pub fn discharging<'a>(
        &self,
        upstream: &'a Upstream,
        id: &ObligationId,
        covering: &[String],
        current: &str,
    ) -> Option<&'a UpstreamAttest> {
        let found = upstream
            .items
            .iter()
            .filter(|u| &u.attest.id == id)
            .find(|u| {
                let trusted = self.trusts(&u.package, &u.attest.role);
                let covered = covering.iter().any(|r| r == &u.attest.role);
                let fresh = u.attest.digest.as_str() == current;
                trace!(%id, package = %u.package, trusted, covered, fresh, "upstream candidate");
                trusted && covered && fresh
            });
        if let Some(u) = found {
            debug!(%id, upstream = %u.describe(), "obligation discharged by trusted upstream");
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{ObligationKind, Verdict};
    use crate::lock::{Date, Hex};

    fn id(path: &str) -> ObligationId {
        ObligationId::new(ObligationKind::Check, path)
    }

    fn shop_lock() -> Lock {
        let mut l = Lock::new();
        for (path, role, digest) in [
            ("acme.shop::Axle.seat", "source", "aa"),
            ("acme.shop::Axle.fit", "tolerance_review", "bb"),
        ] {
            l.insert(Record::Attest(Attest {
                id: id(path),
                role: role.into(),
                by: "ana@shop".into(),
                at: Date::new("2026-09-01").unwrap(),
                digest: Hex::new(digest).unwrap(),
                verdict: Verdict::Holds,
                margin: "0.01mm".into(),
                deps: vec![],
                reason: "ok".into(),
            }));
        }
        l
    }

    fn upstream() -> Upstream {
        let mut u = Upstream::new();
        u.add_package("acme.shop", &shop_lock());
        u
    }

    fn roles(r: &[&str]) -> Vec<String> {
        r.iter().map(|s| (*s).to_owned()).collect()
    }

    // frob:tests crates/cadetta-obligations/src/trust.rs::Upstream.add_package
    // frob:tests crates/cadetta-obligations/src/trust.rs::Upstream.for_id
    // frob:tests crates/cadetta-obligations/src/trust.rs::UpstreamAttest.describe
    #[test]
    fn dependency_lock_attestations_travel_with_the_package() {
        let u = upstream();
        assert!(!u.is_empty());
        let seat = id("acme.shop::Axle.seat");
        let got: Vec<_> = u.for_id(&seat).collect();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].package, "acme.shop");
        assert_eq!(
            got[0].describe(),
            "acme.shop source by ana@shop on 2026-09-01"
        );
        assert_eq!(u.for_id(&id("acme.shop::Nope.x")).count(), 0);
        assert!(Upstream::new().is_empty());
    }

    // frob:tests crates/cadetta-obligations/src/trust.rs::Trust.discharging
    // frob:tests crates/cadetta-obligations/src/trust.rs::Trust.set
    // frob:tests crates/cadetta-obligations/src/trust.rs::Trust.trusts
    #[test]
    fn trusted_roles_discharge_matching_obligations() {
        let u = upstream();
        let mut t = Trust::new();
        t.set("acme.shop", &roles(&["source", "tolerance_review"]));
        assert!(t.trusts("acme.shop", "source"));
        assert!(!t.trusts("acme.shop", "other"));
        assert!(!t.trusts("acme.parts", "source"));
        let seat = id("acme.shop::Axle.seat");
        let covering = roles(&["source"]);
        let hit = t.discharging(&u, &seat, &covering, "aa").unwrap();
        assert_eq!(hit.attest.role, "source");
        // the dependent role must cover the obligation
        assert!(
            t.discharging(&u, &seat, &roles(&["review"]), "aa")
                .is_none()
        );
        // a changed node is not discharged by an old sign-off
        assert!(t.discharging(&u, &seat, &covering, "zz").is_none());
        // only trusted roles discharge
        let mut narrow = Trust::new();
        narrow.set("acme.shop", &roles(&["tolerance_review"]));
        assert!(narrow.discharging(&u, &seat, &covering, "aa").is_none());
    }

    // frob:tests crates/cadetta-obligations/src/trust.rs::Trust.discharging
    #[test]
    fn without_trust_upstream_is_information_only() {
        let u = upstream();
        let seat = id("acme.shop::Axle.seat");
        let covering = roles(&["source", "tolerance_review"]);
        assert!(
            Trust::new()
                .discharging(&u, &seat, &covering, "aa")
                .is_none()
        );
        let mut t = Trust::new();
        t.set("acme.shop", &[]);
        assert!(t.discharging(&u, &seat, &covering, "aa").is_none());
        // still visible as information
        assert_eq!(u.for_id(&seat).count(), 1);
    }
}
