# Verification of cadetta spec draft 1.4 (read-only)

Mechanical checks run (scripts in this directory: xref.py):
- Cross-refs: 190 `<doc>.md N[.M]` references resolved to existing headings; 0 missing. 5 "NOFILE" hits are false positives (wrapped paths, bare emitter names in adapters.md 254-255, README 3 pilots, language.md 19 requirements.md prose).
- Diagnostic codes: every code used anywhere is in lints.md catalog (A276/B209 are part numbers, not codes). No orphans. G001-G007 and R007 in geometry.md 7 match lints.md name and severity.

Severity: blocker 2, gap 11, polish 4.

## Blockers
1. [blocker] language.md:121 bound grammar has no `==`; geometry.md:90-94 uses `shaft.z == mount.z` as an interface bound and says violation is I001, while language.md:197-198 says a violated bound is C001 against the implementer. Fix: add `| path "==" path` to the bound production; in geometry.md:92 replace "I001" with "C001 against that implementer".
2. [blocker] stdlib.md:212-213 says `Sourced` members are optional (missing ones listed by `?` roll-ups), but Sourced is an interface and a missing member is I001 (error) per lints.md:218 and language.md 3.2. Fix: declare members optional (`unit_cost?: Money`, `moq?: ...`), add optional-member syntax to language.md 3.2 and an I001 exemption sentence in lints.md I001.

## Gaps
3. geometry.md:37-38 ScrewJoint omits `into`; stdlib.md:110-113 defaults it to `host.material` but Rail (19-24) has no material. Fix: add `material = Material::Al6061T6` (and the `use`) to Rail.
4. geometry.md:58-59 "Swap to M6 ... re-decided" is numerically wrong (max distance 0.424mm > 0.30mm allowance, 0 reachable, stays C002). Fix: drop the M6 clause or state "tighten a tolerance"/give numbers that decide.
5. config-cli.md:199 `text` row describes `file:line:col: severity[CODE]: message`, which is lints.md:85 `short`. Fix: "rustc style: `severity[CODE]: message`, `--> file:line:col`, source excerpt, `= note:`/`= help:`/`= path:` lines".
6. config-cli.md command table: `ack` (134) lacks `--dry-run` `--as EMAIL` (obligations.md:251); `check` (124) lacks `[--namespace NS] [GLOB...]`; `solve` (125) lacks `[--namespace NS] [GLOB...] [--variants]` (solvers.md:99,111; bindings.md:123). Fix rows: `check [--release] [--as-of DATE] [--watch] [--namespace NS] [GLOB...]`; `solve [SOLVER...] [GLOB...] [--namespace NS] [--variants]`; `ack ... [--dry-run] [--as EMAIL]`. Also solvers.md:99 `[PATH-GLOB...]` vs `[SOLVER...]`: unify.
7. bindings.md:105-109 claims "Every CLI verb has a method" but 111-129 lacks `swap`, `lock`, `cite pin`, `init`, `units`, `why --stale`, `impact --since/--worktree`, `import --update/--write`, and options of ack (baseline, release, dry_run, as), check (as_of, namespace), obligations (release, as_of). Fix: add rows (`project.swap(slot, alternate, reason)`, `project.lock_verify()`, `project.impact_since(rev=None)`, `project.import_(..., update=False, write=False)`, `cadetta.units(kind=None)`) or reword to "Every query and mutation verb except init, lock, cite pin".
8. bindings.md:124 `project.emit(...)` model "sync.json records" but config-cli.md:138 schema is `emit.json`. Fix: `emit.json`. bindings.md:10-15 schema list omits `emit.json`, `ir.json`, `lock.json`.
9. ir.md:101 `Locator.kind: global|dimension|property` omits `frame` (adapters.md:136,153; geometry.md:258). Fix: add `|frame`.
10. SLOT/ALT form inconsistent. language.md:147-148 says canonical slot path and qualified alternate, but language.md:214, 222-223 (`for alt Rover/wheel=Colson2Series`), lints.md:37 (`"alt": "wheel=Colson2Series"`), lints.md:100, ir.md:136 use unqualified forms; adapters.md:225 and config-cli.md:130 use qualified `acme.rover::Rover/wheel`. Fix: use `for alt acme.rover::Rover/wheel=acme.company::Colson2Series` everywhere, or state that the namespace prefix may be omitted inside its own namespace.
11. obligations.md:53-64 kinds table has 8 kinds including `waiver`; decision O4 lists 7. config-cli.md:66 shows only `ots:*` and 3.3 lists no kinds. Fix: confirm `waiver` is intended (lints 2.1 and obligations 252 use `ack waiver:GLOB`), and add a pointer to obligations.md 2.2 in config-cli 3.3. Grep found no other divergent kind list.
12. stdlib.md:131-133 `part screws[4]: ScrewJoint(..., at = holes)` "is an error" with no code. Fix: name a code (e.g. G004/E006) or add a lint.
13. language.md:90-94 reserved words lack `and or not` (G11); grammar 102-145 has no `expr` production for them. Fix: add to reserved list and cite the precedence list at 471.

## Polish
14. language.md:115 `place` grammar allows `for IDENT in ( path | 0..count(path) )`; geometry.md 4 documents only the Count-range form. Drop `path` or document it.
15. geometry.md:102 "`Point3 in origin`" should be `Point3 on origin` (language.md:117, 252).
16. language.md:79-80 example uses `bracket[0].holes` but Bracket (59-61) declares no `holes`. Add `holes: [Point2 on origin; 2] = ...` or mark elided.
17. README.md OQ index OQ-1..OQ-18 matches the OQ entries in the documents; no action.

## Verified consistent (no finding)
Lock records (attest, waiver, synced, synced(file), adopted, swap); three-way sync table and sync flags (J6); mangling rule across adapters 3.1 / sw-equations / json / csv; adapter `id` and bindings/<id>.toml; Value.nominal=designed with drawn?; placement path form; `Rover/axle[*].end_diameter` and R012; protocol methods; waiver grammar and allow(); [defaults] precedence; deferred --watch json / lsp; std.commercial::Sourced members; ScrewJoint one-per-pattern (only the explicit error counter-example remains); rulings R008-R013, O004, W005, E001-E011, G005-G007, diag record, import section, init flags, --as-of present.

## Ruling coverage
Missing or partial: G9 datum bound grammar (1), G11 and/or/not (13), S11 slot/alt naming (10) and verb coverage (7), S7 schema list in bindings (8), O4 kind count (11). All others present.
