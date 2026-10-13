# Pilot round 2 synthesis

Round 2 re-ran pilots 02, 04 and 05 against spec draft 1 and added
pilot 07 (part swap). The reports were capped at 400 words. Detail is
in each `FRICTION.md`; the spec response is draft 1.1
(`docs/decisions/2026-10-13-draft-1-1-decisions.md`).

## Round-1 findings

| Pilot | Resolved | Partial | Open |
|---|---|---|---|
| r2-02 bearing housing | 8 | 2 | 0 |
| r2-04 linear actuator | 8 | 3 | 0 |
| r2-05 SOLIDWORKS migration | 7 | 3 | 1 |

## New findings and responses

| Finding | Pilot | Severity | Draft 1.1 |
|---|---|---|---|
| Swap invalidation undefined: own vs transitive digests | 07 | blocker | closure digests (ir.md 4) |
| Derived values have no per-alternate values | 07 | blocker | variant evaluation, IR `variants` (language.md 3.2) |
| Viability, waivers per alternate, promotion, the old primary | 07 | awkward | language.md 3.2, lints `for alt`, I003 |
| impact output schema and roll-ups | 07 | awkward | obligations.md 5.1 |
| VolumeFlow, kind expressions, compound solver outputs | 07 | awkward | quantities.md 3, language.md 3.10 |
| Derived assembly values lose tolerance | 02 | awkward | `let` analysis values, C004 |
| Alias identity | 02 | awkward | aliases share the variable |
| Matched-lot stacks | 02 | awkward | deferred (OQ-9) |
| A family referenced in its own argument | 04 | awkward | `Type.member` for parameter-free members |
| Lever-arm torque exponent | 04 | awkward | assignment sets the canonical exponent |
| Flooring to a Count | 04 | awkward | `floor`/`ceil`/`round` return Count |
| Several locators per path; ref targets; what is emitted | 05 | awkward | adapters.md 3 |
| SOLIDWORKS format details | 05 | awkward | emitters/sw-equations.md |
| `common_values` hides an imported duplicate | 05 | awkward | D001 revised |
| `#[emit]` not inherited by references | 05 | open since r1 | quantities.md 5.6 |
| std module paths and column names | 04, 05 | awkward | stdlib.md |
| `use` separators, enum variants, check names | 04, 07 | nit | language.md 3, 6 |

## Cross-cutting observation

Five of seven pilots hand-built fastener logic: clearance holes, screw
length, engagement, tap drills and thread callouts. This motivated
`std.fasteners::ScrewJoint` (stdlib.md 3).

## Pilot 07 results (hand-evaluated, illustrative figures)

- **Cost swap** (20 mm fan): blocked. It violates the solver-bound
  airflow requirement (22.9 vs 24.1 m^3/h) and the lead-time check.
  Unit cost would be -21%.
- **EOL and performance swap** (Noctua): viable. One connector-fit
  check is undecided.
