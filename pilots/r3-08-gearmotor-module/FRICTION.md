# Pilot r3-08 friction (gearmotor module, draft 1.2)

## 1. Verdict

Draft 1.2 expresses the design almost fully: variant solving, grip,
H7 on x, max? and policy tables all work. 3 PROPOSED lines in
design.cdt, all in the shop-policy tables (key-only rows, TapClass
variants).

## 2. Prior findings

1. Solver under variants: RESOLVED (solvers.md 4.1, S006).
2. Counterbore grip: RESOLVED (screw 16 mm -> 10 mm).
3. Hole-class one-sided fit: RESOLVED (`H7 on x`).
4. FastenerPolicy shape: PARTIAL (members listed; TapClass, TapColumn and key-only tables undefined).
5. Solver input scope: RESOLVED (member solver sees instances).
6. max/min roll-up: RESOLVED.
7. Screw cost source: PARTIAL (documented as missing; no way to price, finding 2).
8. Negative-both tolerance, Material variants: RESOLVED.

## 3. New findings

1. awkward | language.md 3 vs solvers.md 1 | the `member` grammar omits `solver`, yet solvers.md says a solver may be an assembly member | add `solver` to the member production.
2. awkward | stdlib.md 3-4 | ScrewJoint has no argument to choose the screw family, so "wrap the family with a price table" cannot be written and module_cost never includes screws | add `screw_family` (or `price`) to ScrewJoint.
3. awkward | stdlib.md 4, language.md 3.6 | `Table<TapClass>` with `TapColumn` values: neither enum is listed; key-only table syntax (`lengths`, `drills`) is unspecified | list the variants and show a key-only `table`.
4. nit | stdlib.md 4 | `tap_lead_in: Count` "in pitches": not stated that it multiplies thread.pitch | write `tap_lead_in * thread.pitch`.
5. nit | stdlib.md 2 vs 4 | Iso4762 lists member `unit_cost?` while section 4 says it has no `unit_cost` | drop the member from section 2.
