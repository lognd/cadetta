# Geometry conformance fixtures

Must-fire and must-not-fire pairs for `docs/spec/geometry.md`: the
placement and position diagnostics G001-G007, the datum rules (E001,
E002, E006, C001, C002, S001, I001), the Q006 marks for rotations and
circle patterns, and value fixtures for pattern order, datum
operators, placement composition and the axis checks. Layout and the
expect.json format are in `../README.md`.

- Fixtures with `kind = "source"` and a check entry run today.
  Value (`eval`) and binding entries are written but the runner does
  not execute them yet; that gap is ticket ~6BCVN48.
- `r007-not-from-origin` is a copy of
  `../adapters/geom-r007-not-from-origin.*` (binding entry).
- The bolt-float placement of geometry.md 5.2 is covered by
  `g005-*` and `g008-*`: x translation (`g005-check-looser-than-float`),
  y translation (`g005-float-on-y`), theta (`g005-float-theta-far-datum`),
  `theta = 180deg` when `float >= s` (`g005-float-half-turn`), selection
  among joints (`g005-float-selects-smallest-theta`), locating removal
  (`g005-locating-joint-no-float`), either parent/child order
  (`g005-float-base-is-child`) and the own check reading the nominal
  edge (`g005-own-check-reads-nominal`).
- Covered elsewhere and not duplicated: the CAD binding rules of
  geometry.md 6 in `../adapters/geom-*`, datum variants in
  `../ir/datum-variants-chain`, the floating-nut allowance in
  `../stdlib/check-aligned-nut-floating`, `../language/g001-array-plus-point`
  and `../language/e001-origin-member`.
- The binary lags the spec for most of these; every failing fixture
  is listed in `../known-gaps.toml` against ~H0EKNH2 (G-codes),
  ~S5DC5W0 (datum declarations) or ~MCJWA2D (geometry evaluation).
- Spec decisions made while writing them: geometry.md 8 (ticket
  ~N69K25Y).
