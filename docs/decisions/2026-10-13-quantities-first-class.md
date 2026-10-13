# Quantities are first-class, exact and kinded

Status: current
Owner: cadetta
Decisions: none
Audience: contributors

- Date: 2026-10-13
- Ticket: none (pre-ledger)

## Context

The three problems cadetta exists for (duplication, naming, magic
numbers) are all problems of untyped numbers. CAD values cross unit
systems constantly (inch OTS parts in metric assemblies), and a value
that round-trips through floats may stop matching its source.

## Decision

Every value is a quantity: an exact rational magnitude in SI, a
dimension vector, a kind, and an optional tolerance (quantities.md).
Literals are single tokens (`5in`); bare numbers are errors outside
Count, Ratio and expression factors. Kinds refine dimensions (Torque vs
Energy, Angle vs Ratio) and never convert implicitly. Toleranced
comparisons are three-valued.

## Consequences

- `1in` is exactly `25.4mm`; equality-based lints (D001, D002, R001)
  are exact, not epsilon-based.
- Inexact operations are visible (Q006) instead of silently polluting.
- Every adapter must report precision loss when writing native numbers
  (Q004).
- More upfront work in the kernel than a float-plus-unit-string design.

## Alternatives considered

- Floats with unit strings: simpler, but drift and duplication checks
  would need tolerances of their own, reintroducing magic numbers.
- Dimension-only typing (no kinds): cannot catch torque assigned to
  energy or a ratio used as an angle; lithos-2 found kinds necessary.
- Separating number and unit with whitespace (`5 mm`): permits
  ambiguous parses; lithos-2 made the single token rule and it held up.
