# Pilot 05: SOLIDWORKS migration (gearbox cover, gasket, bolts)

## 1. Summary
The target language is fine (cleaned.cdt reads well, the fit/tolerance/`@emit(mid)`
story for 12.012 is the best part). The migration path is where it breaks:
observed IR carries no expressions yet the equations file is all expressions;
`was` cannot cross components; there is no emitter output spec (ordering,
references vs values, tolerance annotation); and D/M lints either cannot run on
observed IR or are silenced by the importer's own provenance.

## 2. Findings
**F1 blocker - ir.md 1 vs adapters.md 3.** Observed IR has "no expressions", but
`"D1@Sketch1@Gasket.SLDPRT" = "PlateL"` IS an expression and import must recover it
or the scaffold is 38 literals. I wrote imported.cdt with references (PROPOSED).
Change: "Observed Param MAY carry `expr` when the native format exposes it (sw-equations always)."

**F2 blocker - language.md 3.3 grammar `was IDENT`.** Merging `Globals.plate_thk` into
`CoverPlate.thickness`, or `Globals.num_bolts` into `GearboxCover.bolt_count`, needs a
path. Bindings are keyed by path so the orphaned `Globals.*` bindings (15) are
silently dead; no lint covers "binding key matches no IR path".
`thickness = 0.25in was d1_boss_extrude1 was Globals.plate_thk`; add a code (R005 orphan-binding).

**F3 blocker - adapters.md 4/6, no emit format.** Unspecified: line order, global
vs dimension, flat value vs reference (`"D5@..Gasket" = "Iso273_m6_clearance"`), header,
doc-comment mapping, whether dimension-bound params also get a global. I picked
sort-by-path, bound-only-at-locator, flat values: SW users lose every live link and the
5 mangled globals are dead. Change: binding key `emit = "ref" | "value"`; spec an ordering.

**F4 awkward - quantities.md 5.2 / lints R001.** Emitting 12.009 (mid) means a read-back
sees 12.009 vs design nominal 12: R001 fires falsely unless drift compares at the
emit point. Also tolerance is lost in the SW file entirely (only a number); I wished
for trailing `' 12mm H7, emitted at mid`. Also the migration itself moves the model
12.012 -> 12.009; nothing reports "emitted value differs from legacy value".

**F5 awkward - ir.md 1, lints D001/D002.** D rules need design IR, so the most valuable
adoption lint (the duplicated 100mm/6.6mm/12.012mm) cannot run on observed IR. D001
excludes same-component pairs, so `Plate_Thk`=6.35mm vs `plate thickness`=0.25in (exactly
equal, 127/20000 m) is never caught once import puts all globals in one `Globals`.

**F6 awkward - lints M001 / language.md 3.1.** Import writes component `source` and carries
legacy `'` comments as `///`; both exempt M001, so 12.012mm is NOT flagged on imported.cdt
(" pilot" is a doc comment). Provenance should not count as explanation. Say whether
component-level source exempts params.

**F7 awkward - D002 direction/coincidence.** "Exactly twice" fires on pure coincidences:
120=2x60, 10=2x5 (across components), 12=2x6. Three waivers needed; direction (which
literal is flagged) unspecified, so waivers may be W004. Restrict to same component or
require a dimensional relation.

**F8 awkward - quantities.md 4.3.** `thickness / 2` is anonymous kind; M001/precision
keyed by kind then misses it. I wrote `cbore_depth: Length = ...`. Scalar scaling should keep kind.

**F9 awkward - quantities.md 5.1.** `head_dia + 1mm` inherits head_dia's `+0/-0.22`; a
derived counterbore should not. No `nominal(x)`/`untol(x)`. I used a literal `11mm H13`.

**F10 nit - emit point not inherited.** Gasket.pilot_bore = CoverPlate.pilot_bore needs its
own `@emit(mid)`; forgetting gives 12.000 vs 12.009 with no lint.

**F11 nit - adapters.md 3 mangling.** `A_b.c` and `A.b_c` both -> `A_b_c`; not injective. Bare
`80`/`60` in the SW file need `--assume-unit` (undocumented); component class (ots vs model)
and `Globals` name are invented by import. `.SLDPRT` vs `.Part` suffix unverified.

## 3. Hypothetical `cadetta check`
- Observed IR (before import): R002 x38 (useful, but one line says "import"); M001 x2 on
  pilot_bore, d7_sketch1 (useful, if not silenced, F6); N003 x2 bolt_dia_mm, bolt_len_in (useful).
- imported.cdt: D001 100mm, 6.6mm, 12.012mm (warn, useful), 5mm (info, useful); D002 on
  plate_l (120=2x60) and bolt_head_dia (10=2x5), noise; N003 x2; M001 x0 (wrong, F6).
- cleaned.cdt: D001 none; D002 x3 waived (noise); N none; M003 none (all check operands
  toleranced); C001/C002 none. R001 on pilot_bore after sync (wrong, F4).
- Not caught by any lint: two diverging copies in one component; orphan bindings; legacy
  value silently moved 12.012 -> 12.009; Bolt.length 0.75in is not a standard M6 length
  (would need a "value not in vendor table" concept); emit-point drift between plate and gasket;
  unit assumption on bare 80/60.

## 4. Evaluation trace
1. thickness = 0.25in = 1/4 * 127/5000 = 127/20000 m. cbore_depth = 127/40000 m = 3.175mm,
   emitted `3.175mm` (exact at 6 decimals, no Q004).
2. thread_depth = 381/20000 - 127/20000 - 3/2000 = 254/20000 - 30/20000 = 224/20000 = 7/625 m
   = 11.2mm. Matches legacy 19.05-6.35-1.5. Mixed in/mm needs no care: good.
3. pilot_bore 12mm H7: lower 0, upper +18um; limits 12.000..12.018mm; mid = 12.009mm =
   12009/1000000 m. Check 1: lhs cbore_dia [11.000,11.270] > rhs head_dia [9.78,10.00]:
   11.000 > 10.00, `holds`.
Ambiguities: H13 band for 6.6mm (6..10 band, +0.22) relies on tables not in spec; "mid" of
a toleranced reference through `=` (does the reference carry the attr?); D002 direction.
