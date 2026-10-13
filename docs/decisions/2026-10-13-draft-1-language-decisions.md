# Draft 1 language decisions after pilot round 1

Status: current
Owner: cadetta
Decisions: none
Audience: contributors

- Date: 2026-10-13
- Ticket: none (pre-ledger)

## Context

Six pilots (`pilots/SYNTHESIS.md`) exercised spec draft 0. The owner
then decided several open points directly. This record closes them
together; the normative text is in `docs/spec/`.

## Decision

1. Source files use the `.cdt` extension (closes OQ-5).
2. The default emitted tolerance point is `nominal`; `#[emit(...)]`
   overrides it per member and `#![emit(...)]` per scope.
3. `@` is reserved for citations; attributes use Rust syntax `#[...]`
   with comma lists, inner `#![...]` scope defaults, and a trailing form.
4. Tolerances are a bracketed postfix operator (`8mm [+0 -0.008mm]`);
   ISO classes stay bare (`12mm H7`).
5. Parameters store design tolerance; checks evaluate analysis tolerance.
   Tolerance evaluation is affine with one source variable per toleranced
   member per part instance (per-instance, owner decision).
6. Magnitudes are `r * pi^k`; kinds track an angle exponent; Torque is
   N*m per radian.
7. The obligation graph is core functionality, not an add-on.
8. Solver outputs are values; the taught idiom is a bound (a check), and
   driving use is supported and explicit.
9. Parts are swapped through interfaces with approved alternates that
   are verified continuously. Reasons to swap (cost, lead time, end of
   life, performance, compliance) are data on implementers.
10. Packages may declare kinds of an existing dimension (closes OQ-3).

## Consequences

- The quantity kernel grows: pi exponents, affine forms, angle tracking.
- The language gains abstraction (fn, enum, match, tables, interfaces,
  parameterized components), so the parser and sema epics grow.
- New spec documents: obligations.md, solvers.md. Tickets must be
  re-planned against draft 1.

## Alternatives considered

- Naive interval arithmetic: double-counts repeated terms (pilot 02).
- Per-class tolerance variables: simpler, physically wrong for two
  instances of one part.
- Driving-by-default solvers: every re-analysis would churn geometry and
  CAD files.
