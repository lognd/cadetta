# Diagnostics under recalibrated lints.md (judgement: useful / noise / wrong)

## Observed IR (read of legacy-equations.txt, before import)
- R002 x38: useful as a count, noise as 38 lines; group per file.
- Q001 x2 (PlateW, BoltSpacingY): useful, but interplay with --assume-unit unspecified.
- D001 x5 groups: 6.35mm vs 0.25in (warn), 100mm (warn), 12.012mm (warn), 6.6mm (info), 6mm bolt dia vs head height (info, noise). Expressions in observed IR let it tell literal from reference: useful.
- M001 x2 (12.012mm in PilotBore and Gasket D7): useful, if observed components count as `model` (unstated).
- N003 x2 (BoltDia_mm, BoltLen_in): useful.
- 5mm Gasket D3 duplicate: not reported (common_values): wrong, a real duplicate.

## imported.cdt
- D001 as above (same five groups; 5mm silent): 4 useful, 1 noise, 1 wrong-silence.
- M001 x2: useful (provenance no longer exempts).
- N003 x2: useful.
- R002/R005: none (bindings accepted). No Q001 (units assumed, @assumed).

## cleaned.cdt
- D001, D002, M001, N, M003, U001, R002, R005: none. Zero waivers (round 1 needed 3).
- C001/C002: none; both checks hold (11 > [9.78,10]; 6.6 > [5.82,6]).
- R006 info x2 on first sync (CoverPlate.pilot_bore, Gasket.pilot_bore 12.012 -> 12.009): useful. Per path or per binding (3) is unstated.
- R001 after sync: none (compared at emit point): correct.
