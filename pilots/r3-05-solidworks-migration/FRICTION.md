# Pilot r3-05: SOLIDWORKS migration vs draft 1.2

## 1. Verdict
Draft 1.2 expresses the migration in full; no blockers. std tables replace the
hand-typed ots, and aliases now emit as refs to the plate's globals (no
`Gasket__*` globals). 2 PROPOSED lines, both in bindings/sw-equations.toml
(none in .cdt). Not applicable here: interfaces/alternates, roll-ups, solvers.
Input: ../05-solidworks-migration/legacy-equations.txt (referenced, not copied).

## 2. Prior findings (r2)
- F10 `#[emit]` not inherited: RESOLVED (quantities.md 5.6; Gasket.pilot_bore emits 12.009 with no repeat).
- F11 `--assume-unit`, `Globals` owner, `.SLDPRT`: PARTIAL (unit handling in adapters.md 3; file-level globals owner still unspecified).
- N1 several locators per path: RESOLVED (adapters.md 3; TOML spelling is not, see N2).
- N2 which global a ref points at: PARTIAL (alias and own global fixed; two globals unspecified, see N1 below).
- N3 SW format: RESOLVED (emitters/sw-equations.md).
- N4 D001 vs common_values: RESOLVED.
- N5 std paths/columns: RESOLVED (stdlib.md 2).
- N6 Q001 vs assume-unit: RESOLVED (read never raises Q001).

## 3. New findings
1. awkward, adapters.md 3: a `ref` dimension on a path with two globals (Plate_Thk, "plate thickness") is ambiguous. Fix: `ref_to` on the locator, or first in file order.
2. awkward, adapters.md 3 / ir.md 3: bindings TOML spelling for several locators is unspecified (IR shows a list). Fix: give one example in config-cli.md.
3. awkward, stdlib.md 2 / quantities.md 5.3: std screw tables are untoleranced, so C001 checks lose the old [9.78,10] head margin; unclear if `Iso4762` accepts non-preferred lengths (0.75in). Fix: tolerance columns; state length is unchecked.
4. awkward, language.md 3.1, 4 / quantities.md 5.4: `CoverPlate.thickness` read from `Bolt` with no instance; which source variable? Fix: state that it is per-assembly-instance or forbid outside parameterized types.
5. nit, emitters/sw-equations.md 1: "authored literal when it differs from the written value" is vague (0.25in vs 6.35mm); dependency level across `let` and unemitted paths is also unstated.
6. nit, stdlib.md 2: the `std.retaining` table row sits after prose and renders outside the table.
