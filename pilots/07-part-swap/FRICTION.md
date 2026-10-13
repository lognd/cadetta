# Friction, pilot 07 (part swap: 80mm fan)

## 1. Verdict
Draft 1 expresses the interface, `alt`, commercial/lifecycle data, a
solver bound and an adaptive bracket well. The swap queries (compare,
impact --swap) cannot yet be produced deterministically: digest scope,
per-alternate derived values and promotion rules are under-specified.

## 2. Round-1 findings
(none: new pilot)

## 3. New findings
1. blocker, ir.md 4 vs obligations.md 2: check digests hash own text, yet
   staleness means "anything it reads changed"; attestations-invalidated
   is undefined. Fix: state that node digests are Merkle over `deps`, and
   the granularity (member, not instance).
2. blocker, language.md 3.2 / ir.md 3: only checks get per-alternate
   verdicts; derived values (screw_length, cutout_dia) and part args
   downstream of an alternate have one value. Fix: define variant
   evaluation (substitute the part, re-evaluate dependents) and put it in
   the IR as `values_by_variant`.
3. awkward, language.md 3.2 / lints.md 2: undecided alternates, waivers
   scoped to one alternate, and promotion of a violated `require` are
   unspecified; NEW is already in `alt` and the old primary's fate is
   unstated. Fix: viable = no violated; `waive ... for alt X`; swap
   rewrites `alt` explicitly.
4. awkward, quantities.md 3-4, solvers.md 1: no `Volume/Time` kind, `kind`
   cannot alias a kind expression, solver `out` kinds cannot be compound,
   no CFM or mmH2O; table `at_least` call and `sqrt(1 + x)` on
   dimensionless anonymous kinds unspecified. Fix: ship VolumeFlow and
   `kind K = <expr>`; specify lookup functions.
5. awkward, obligations.md 5: output schema, ordering and roll-ups
   (lead time max, mass(self) failing on parts without mass) unspecified.
   Fix: add `docs/schemas/impact.json` and a `max?` roll-up.
6. nit, lifecycle/solvers: `#[lifecycle(eol, date)]` date meaning, and
   solver `check` written without NAME in solvers.md 5 vs language.md 6.
