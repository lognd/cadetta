# Pilot r3-04: linear actuator, friction (draft 1.2)

## 1. Verdict
Draft 1.2 expresses the actuator almost completely: class-level `TrScrew.cut_increment`, `floor`, `as Length`, std `Material`/`Materials` and `let` stack-ups all work. 1 PROPOSED line (ScrewJoint into a vendor-tapped hole). No blockers.

## 2. Prior findings
- r2-1 family in own argument: RESOLVED (`TrScrew.cut_increment`).
- r2-2 lever-arm exponent: RESOLVED (4.3 states it).
- r2-3 floor to Count: RESOLVED (`floor(x) + 1`).
- r2-4 `/ 1rad`: RESOLVED (`as Length`).
- r2-5 use separators and enum variants: RESOLVED (3.1 bare variants, grammar).
- r2-6 std member names: PARTIAL (tables listed; `Iso4762`/`ScrewJoint` members `screw_length`, `grip` ok; `sum(array.member)` defined).
- r1 cross-repo packages: OPEN (OQ-11).

## 3. New findings
1. awkward, stdlib.md 3: ScrewJoint assumes we tap the hole (tap_drill, drill_depth, callout, depth_available); no way to say the threaded hole is vendor-made with fixed depth. Fix: `tapped_depth` arg or `tapped = vendor`.
2. awkward, lints.md F004 vs C002 / language.md 3.7: a fit whose clearance partly overlaps its required range is F004 (error, waivable) or C002 (undecided)? r2 waived C002. Fix: say F004 fires only when disjoint and C002 otherwise, or the reverse.
3. awkward, language.md 4 D003: is the cycle member-level or component-level? `motor_mount` takes `motor_screws[0].clearance_hole` while the joint reads `motor_mount.thickness`. Fix: state D003 is per member.
4. nit, quantities.md 4.1: `sqrt` of Area (`sqrt(2 * x^2)`) has no kind rule (Length?). Fix: add sqrt/cbrt to the kind algebra.
5. nit, language.md 3.3/3.5: can a `let` feed a part argument (`screw_min_length` into `ceil_to` into `TrScrew(length)`) and be read across a part (`drive.thrust`)? Implied by "readable like a parameter" but not stated for arguments or other components.
6. nit, stdlib.md 2: the `Din471` row sits after the `Material` paragraph, outside the table; `Material` variants have no per-variant `face`-style note on how a vendor part states its material (needs `Material` import in ots).
