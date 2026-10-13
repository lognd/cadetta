# Pilot synthesis (round 1)

Six pilots against spec draft 0: 01 bracket (simple), 02 bearing housing,
03 Pi enclosure, 04 linear actuator (complex), 05 SOLIDWORKS migration,
06 imperial extrusion frame. Each finding below names the pilots that hit
it; a finding hit by three or more is structural, not a corner case.

What held up: exact rationals (mixed in/mm chains came out exact with no
care), three-valued checks (useful C002s on real fit risks in 02 and 04),
canonical paths, citations-as-provenance, and the brace surface syntax.
What broke: the quantity core at its edges (literals, tolerance syntax,
angles, interval semantics) and the lack of abstraction (tables, families,
arrays, functions). Lints are noisy in exactly the places a real design
is dense with coincidental values.

## A. Literals and tolerance syntax

| # | Finding | Pilots | Resolution |
|---|---|---|---|
| A1 | `3/4in` lexes as `3 / (4in)`: silent wrong dimension | 06 | Fraction literals are one token: `3/4in`, `1-1/2in`, `1/16"`. Mixed numbers need no spaces; `3 - 1/2in` with spaces is subtraction. |
| A2 | `2'6"` is two tokens and a parse error | 06 | Compound literal of descending units, one token: `2'6"`, `2ft6in`, `1h30min`. |
| A3 | `8mm +0mm -0.008mm` also parses as arithmetic (7.992mm) | 02, 06 | Deviations go in a postfix bracket: `8mm [+0 -0.008mm]`, `10mm [+-0.05mm]`. ISO classes stay bare postfix: `12mm H7`. |
| A4 | `+-` cannot follow an expression or take a reference | 04, 06 | The bracket is a postfix operator on any expression: `round_to(x, 1/32in) [+- Extrusion.cut_tol]`. |
| A5 | One-sided limits (ISO 4762 dk max, bearing d_a min) | 01, 02 | Bound declarations: `head_dia <= 8.5mm`, `abutment_dia >= 9.6mm`, `head_dia in 8.28mm..8.5mm`. A bound is a requirement, not a value; it cannot be emitted without an accessor. |

## B. Tolerance semantics

| # | Finding | Pilots | Resolution |
|---|---|---|---|
| B1 | Vendor tolerances leak into designed parts (`head_dia + 1mm` inherits `-0.22`) | 01, 02, 04, 05 | **Parameters hold design tolerance; checks evaluate analysis tolerance.** A `model` parameter assigned from arithmetic stores the nominal only, plus whatever tolerance it explicitly attaches. A pure reference (`= fit.shaft`) keeps the referent's tolerance. Checks evaluate over the full tolerances of the params they read. This is also the correct worst-case analysis: a stack uses each part's drawing tolerance. |
| B2 | No way to take nominal, min, max | 01, 02, 04, 05 | Accessors on any quantity: `.nominal .min .max .mmc .lmc` (untoleranced results). `x.max` replaces the "nominal = max" hack. |
| B3 | Naive intervals double-count correlated terms (false C002) | 02 | Evaluate tolerances in affine form: each toleranced source parameter is one variable, linear ops stay exact, nonlinear ops fall back to intervals and mark the result conservative. Default correlation unit: one variable per class parameter (OQ: per instance). |
| B4 | `round_to` undefined on toleranced/inexact input, ties, ceiling | 03, 04, 06 | `round_to`, `ceil_to`, `floor_to`: result is an exact rational; rounds the nominal and keeps deviations; ties away from zero; on an inexact input within 1e-9 relative of a tie it is an error. Rounding restores exactness (clears Q006). |
| B5 | `mmc/lmc` emit needs a hole/shaft role | 02, 05 | Role comes from the ISO letter case, the `fit` side, or `#[role(hole)]`; it survives pure references. |
| B6 | Fits only take ISO 286 on both sides | 01, 02, 03, 04 | `fit` takes any two sides; each is an ISO class or a path: `fit seat = j5 on Bearing.bore`, `fit insert = Case.insert_hole / Insert.od`. The fit type and min/max clearance are computed; `require` a range: `fit insert ... [-0.4mm .. -0.1mm]`. |

## C. Angles

| # | Finding | Pilots | Resolution |
|---|---|---|---|
| C1 | `deg`, `rev` are not exact rationals (pi) | 04, 06 | Magnitudes are `r * pi^k` with rational `r`. `360deg / 1.8deg == 200` exactly; pi cancels. Only trig and roots go inexact. |
| C2 | Dimensionless Angle hides missing `2*pi` (wrong by 6.28x, silently) | 04 | Kinds track an angle exponent. `lead: Length/Angle` is legal and natural (`8mm/rev`). Dropping an angle exponent implicitly is Q008 (warning), fixed by an explicit `/ 1rad`. |

## D. Abstraction

| # | Finding | Pilots | Resolution |
|---|---|---|---|
| D1 | Standards tables have no form; the M5 to M6 swap leaves the hole wrong | 01, 05 | First-class `table` with typed keys; std ships ISO 273, 4762, 286, DIN 472 first: `Iso273[screw.thread].medium`. Lint T001 when an `ots` value is not a row of its cited table. |
| D2 | Part families (screw lengths, rail lengths) | 01, 04 | Parameterized components: `ots Iso4762(thread: MetricThread, length: Length) { ... }`, `part s: Iso4762(M5, 25mm)`. |
| D3 | Helpers, conditionals, enums | 03, 04, 06 | Rust-flavoured, total: pure `fn` with kinded signatures, `if` expressions, `enum` + `match`, `let` in blocks. No loops, recursion, mutation, closures. |
| D4 | No points, arrays, patterns, instance counting | 01, 03, 06 | `Point2/Point3` structs, fixed arrays `[T; N]`, `part leg[4]: Leg`, `count(...)`, `sum(...)`, quantified checks `check for b in [a, b]: ...`. Coordinate frames deferred (OQ). |
| D5 | Strings and materials have nowhere to live | 01, 04, 06 | `Text` kind and enums: `material = Material::Al6061T6`. |
| D6 | Process compensation (FDM) | 03 | `process` declarations with role rules; `#![process(Fdm)]` on a model, `#[hole]` on a feature. The IR keeps intent and drawn value; emit writes drawn. Lint P001: feature under a process with no role. |
| D7 | `ots` cannot derive or be measured | 03, 04 | `ots` may use expressions over its own params (the drawing states pitch and positions). Citation kinds `@measured`, `@assumed` replace "ots for measured facts". |
| D8 | Physics models bind to classes, not instances | 04 | Falls out of D2: `part drive: Drivetrain(motor = motor, screw = screw)`. |
| D9 | Namespaces, imports, packages unspecified | 02, 04 | Many files per namespace; `use acme.parts::Bearing608`; a package mechanism for shared parts libraries (deferred to its own spec). |

## E. Lint recalibration

| # | Finding | Pilots | Resolution |
|---|---|---|---|
| E1 | D001 noise on coincidental round values; misses same-component copies | 02, 04, 05, 06 | Score by specificity (significant digits; `12.012mm` is strong, `8mm` is weak); compare model-vs-anything, never ots-vs-ots; include same-component pairs; run on observed IR too. |
| E2 | D002 "exactly twice" fires on coincidences | 03, 05 | Narrow D002 to "literal equals an available fit member, table cell or accessor" (e.g. `12.009mm` equals `wheel_axle.shaft.mid`). |
| E3 | M001 silenced by import's own provenance and doc comments | 05 | In `model`, only a tolerance, fit, table lookup or waiver explains a value. In `ots`, a citation does. |
| E4 | M003 fires on capacity checks | 01, 02, 04 | Only parameters that participate in a `fit`. |
| E5 | Q006 fires transitively | 04, 06 | Root cause only, with the dependents counted in the message. |
| E6 | Waivers are one code, one path | 04 | Globs: `waive D001 on Bearing608.*`. |
| E7 | New codes | all | Q008 angle-dropped, Q009 non-integer-count, R005 orphan-binding, R006 adopt-value-change, S003 solver-stale, T001 not-in-table, P001 missing-process-role, U001 unused-parameter. |

## F. Import and emit

| # | Finding | Pilots | Resolution |
|---|---|---|---|
| F1 | Observed IR "has no expressions" but SW equations are expressions | 05 | Observed params MAY carry `expr` when the native format exposes it. |
| F2 | `was IDENT` cannot cross components | 05 | `was` takes a path: `was Globals.plate_thk`. |
| F3 | Emit format unspecified (order, refs vs values, tolerance notes) | 05 | Every emitter spec states: ordering (dependency order, then path), refs vs values (keep live references where the target supports them), tolerance as a trailing comment, a header carrying the IR digest. |
| F4 | Drift fires at mid vs nominal | 05 | Drift compares against the emitted point, not the nominal. |
| F5 | Name mangling not injective | 05 | Mangling is `Component.param` where the target allows dots; otherwise `Component__param` and N005 rejects `__` in names. |

## G. Things no lint catches (record, do not chase yet)

Wrong sign in an interference expression (02); missing cutouts or
connectors in a list (03); forgetting screw starts (04); backdriving and
critical-speed physics (04); stock-length availability (04, 05); plywood
nominal vs actual (06). Most of these become catchable once tables (D1)
and fits-as-relations (B6) exist; the rest are engineering judgment.
