# Pilot r2-05: SOLIDWORKS migration vs draft 1

## 1. Verdict
Draft 1 fixes most of round 1: the migration now runs end to end (expressions
become references, `was` merges globals, tolerance and emit point round-trip).
What remains is emitter detail: SW globals vs dimensions, many natives per path.
Input: ../05-solidworks-migration/legacy-equations.txt (unchanged).

## 2. Round-1 findings
- F1 RESOLVED (ir.md 1: observed IR carries expressions).
- F2 RESOLVED (`was Globals.x` across components; R005 exists, silent here).
- F3 PARTIAL (3a + `mode` exist; see N1, N2, N3).
- F4 RESOLVED (R001 vs emit point; trailing tolerance comment; R006).
- F5 PARTIAL (D001 now runs on observed IR and in-component; see N4).
- F6 RESOLVED (M001: provenance and docs do not explain).
- F7 RESOLVED (D002 narrowed; zero waivers needed).
- F8 RESOLVED (4.1 rule 3 keeps Length).
- F9 RESOLVED (5.3: `head_dia + 1mm` stores nominal).
- F10 OPEN (`#[emit]` not inherited by pure references; repeated, PROPOSED).
- F11 PARTIAL (`__` mangling injective; `--assume-unit`, `Globals` owner, `.SLDPRT` still unspecified).

## 3. New findings
1. awkward, adapters.md 3: "one table per path" but legacy binds one value to a global AND a dimension (Plate_Thk, "plate thickness", D1@Boss-Extrude1 all land on CoverPlate.thickness). Fix: allow several locators per path; `ref_to` names the global.
2. awkward, 3a: for `mode = ref` the spec does not say which global a dimension points at (own mangled one, a bound global locator, or the referent of a pure alias), whether derived params emit their expression or value, or whether unbound params are emitted. I chose own/first global, flat values, bound only.
3. awkward, 3a: SW format unspecified: dots in global names, multi-line doc comments, tolerance plus citation in one trailing comment, quoting, whether "dependency order" means anything when values are flat.
4. awkward, lints D001: `common_values` hides the real Gasket 5mm duplicate; "significant digits" of 100mm unclear; one diagnostic per literal or per group unstated. Fix: ignore `common_values` when the other side is an imported literal.
5. awkward, language.md 3.6 / stdlib: no module path or member names for shipped tables and fasteners (`use std::tables::Iso273` is PROPOSED); literal citations (`@"ASTM..."`) vs S002 reachability.
6. nit, Q001 vs `--assume-unit`: bare 80 and 60 at `read` are Q001 (error, fixed) or exact:false? Unstated.
