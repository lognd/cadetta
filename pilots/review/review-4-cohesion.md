# Review 4: cohesion of the whole spec

Summary
- Development 4/5 for cross-references: every "x.md N" pointer I checked resolves, and the diagnostic catalog is complete (every code used anywhere is in lints.md, no orphans). Cohesion is weaker where docs promise a chain (language -> IR -> lint -> CLI -> schema -> Python) and a link is missing.
- Most important fix: settle the `require`/waiver/attestation contradiction (finding 1), then give every obligation kind a diagnostic and a selector (finding 3).
- Checks done: codes (all 80 in lints.md; G001-G004 and R007 are duplicated verbatim in geometry.md 7 and agree); OQ-1..OQ-18 are unique with no gaps.

## Findings

### 1. [blocker] `require` waiver rules contradict each other
- language.md 6: a `require` "cannot be waived without an attestation". obligations.md 4: "Attestations are the only way to waive a require". lints.md 2 and W003: a `require` is never waived (waiver is an error). obligations.md 5.1: "a violated `require` cannot be waived". language.md 3.2: a violated `require` makes an alternate non-viable "whatever is waived".
- Also unclear: obligations.md 2 lists state `waived`, but no row says what an attestation does to a `violated` require (C001 stays an error?).
- Fix: one rule in obligations.md 4, linked from the other four places: "A `require` is never suppressed. An attestation discharges only the sign-off obligation of a `holds` require (O003); a `violated` require stays `violated` and C001, and no waiver or attestation changes that. The state `waived` applies to `waive`d diagnostics only." Reword language.md 6 and obligations.md 4 sentence to match.

### 2. [blocker] Process compensation: `nominal` means drawn in the IR but designed everywhere else
- ir.md 3: "`designed` holds the value before process compensation; `nominal` is the drawn value". language.md 3.8: fits, checks and every read "see the designed value", `x.drawn` reads the drawn value. quantities.md 5.2: `.nominal` is the nominal. A consumer reading `Value.nominal` (the Python `d.nominal`, a JSON consumer, a lint) silently gets the drawn value for compensated members, the opposite of what language semantics say.
- Also unspecified: how the emit point (quantities.md 5.6, `mid`/`mmc`) composes with the drawn value (is mid taken of the designed tolerance, then compensated?), and what tolerance the drawn value carries ("exact", language.md 3.8, but R001 compares to "the emitted point").
- Fix: `Value.nominal` is always the semantic (designed) value; add `Value.drawn?: Exact` only when a process rule applied. Delete `designed`. State in quantities.md 5.6: emit point is chosen on the designed value, then the rule is applied to that point, and that is what is emitted and what R001 compares against. Update sw-equations.md comment rule accordingly (the comment already shows the authored literal).

### 3. [gap] Obligations have no consistent diagnostic and no selector grammar
- obligations.md 2 lists eight obligation kinds, lints.md has O001-O003 plus S003/S004/R001. Unmapped: "CAD file reflects design" stale-by-digest (R001 is value drift, a different thing), "as-built conforms". Overlap: a changed cited file raises S004 and also O001 (a `source` attestation went stale) with no rule for which wins. Solver staleness is S003 only, so `required_for` roles cannot gate it.
- `SELECTOR` (`cadetta ack SELECTOR`, `covers = ["fit:*", "check:*", "ots:*"]`) is never defined, and it is not a canonical path glob as used in `waive`.
- "As-built conforms" (obligations.md 2, node "inspection data sets") has no ingestion path: OQ-13 is open, no adapter, CLI verb or lint exists, and the scope list in README omits it. A table row presented as normative cannot work.
- Fix: add a table column "Diagnostic" to obligations.md 2 (stale CAD sync -> new `R008 cad-stale`, off by default; citation stale -> S004 only, O001 reserved for role attestations and not raised when S004 fires for the same node; solver -> S003). Define selector in obligations.md 4: `KIND:PATH-GLOB` with KIND in `ots model assembly fit check require solver citation`, e.g. `fit:acme.rover::Axle.*`. Mark the as-built row "0.2.0, see OQ-13" and drop the node kinds that only it uses.

### 4. [gap] IR schema (ir.md 3) is missing entities and fields other docs rely on
- `Project` lists `interfaces[]`, `tables[]`, `waivers[]` but no `Interface`, `Table`, `Waiver` entity is defined; processes (class `process`) have no rule/role fields; `Component` has no `datums[]`, `points[]` or `placements[]` although geometry.md 6 says "the IR holds each datum... each placement"; attestations and obligation states are not IR at all, yet `cadetta obligations --format json` must return them.
- Observed IR uses `exact: false` plus a reason, `unit_assumed` (adapters.md 3, 4.4, sw-equations 3) and native expressions; `Value`/`Member` have no `exact`, `reason`, `unit_assumed` field. `Binding.locators` lacks `ref_to`, which the bindings.toml example in adapters.md 3 uses.
- Fix: add to ir.md 3: `Interface {path, members[], bounds[], datums[]}`, `Table {path, key, cols[], rows[], citation?}`, `Waiver {code, glob, because, expires?, for_alt?, targets_closure}`, `Component.datums[]/points[]/placements[]`, `Process {rules[{role, expr}], params[]}`; on `Value`: `exact: bool`, `reason?: string`, `unit_assumed?: string`; on `Locator`: `ref_to?: string`. Put attestations and obligation state in a separate `cadetta.obligations/1` schema (see finding 7).

### 5. [gap] Geometry cannot reach CAD: datum offsets and placements have no canonical path
- geometry.md 6 promises that datum offsets bind to reference geometry and placement offsets to mate dimensions (`D1@Distance2@Mount.SLDASM`), and that tolerated offsets "become source variables". But ir.md 2 gives paths only for `Type.member` and `/part[index]`; a `Datum`/`Placement` has no member path for its x/y/z (the `Point` section says only point coordinates are also Members), and ir.md 2 cannot express `Bracket.holes[1].x` (index on a member) or `Axle.seat.shaft` in its own grammar (they appear only in examples). Without paths there is nothing for bindings, drift lint (R001), waivers, or `why` to attach to.
- Fix: extend ir.md 2 grammar: `member = IDENT { "." IDENT | "[" index "]" }`; define datum offset paths `Type.<datum>.offset.x` (rotation: `.rotate.z`) and placement paths `Assembly/<part>.place.x` (one per op axis); require each to be a Member with `provenance: authored`. State that `Placement.ops` entries are Member refs, not inline Values.

### 6. [gap] Many documented errors have no diagnostic code
- lints.md opens with "Every diagnostic has a stable code", yet the specs name errors with no code and no family for syntax/resolution: table lookup miss (language.md 3.6, "an error at evaluation"), missing table column or kind mismatch (3.3), `Type.member` reading a parameter-dependent member (3.1), `mmc`/`lmc` without a role (quantities.md 5.6), non-exhaustive `match` (3.8), part count not a Count or not known before evaluation (3.5, only D003 hinted), arithmetic dimension mismatch inside tolerance deviations (5.1), wrong argument type at a `part` site, parse errors, unresolved names, a solver `in` expression that does not resolve. `cadetta explain CODE` cannot work for them.
- Fix: add family `E` to lints.md (all fixed): E001 syntax-error, E002 unresolved-name, E003 argument-mismatch, E004 lookup-miss, E005 table-shape-mismatch, E006 non-exhaustive-match, E007 role-required (mmc/lmc), E008 parameter-dependent-type-read. Each referenced from the owning section. A variant-evaluation E004 makes the alternate non-viable (I002) as language.md 3.2 already says.

### 7. [gap] Capability chain is incomplete from CLI to JSON schema to Python
bindings.md 1 says Python returns "exactly what the CLI's JSON returns" for every query; the actual coverage is:

| Capability | CLI | JSON schema listed | Python |
|---|---|---|---|
| check / diagnostics | yes | "diagnostic schema" (not in any schema list) | `check()` |
| why, impact, impact --swap, compare | yes | yes | yes |
| obligations | yes | no | no |
| ack (writes lock) | yes | no | no |
| solve | yes | `cadetta.solve/1` is the solver protocol, not the verb's report | no |
| ir (design / observed) | yes | yes | `evaluate()` only; no observed |
| emit / sync / import | yes | no (sync report, import proposal) | no |
| explain, units | yes | no | no |
| fmt | yes | n/a | `format_source` only |
- A Python user can run the whole loop only through subprocess. `--assume-unit` (adapters.md 3) belongs to no listed command; `import`'s "the user accepts them" has no flag or file.
- Fix: in config-cli.md add a column "Output schema" to the command table (`check.json`, `obligations.json`, `ack.json`, `sync.json`, `import.json`, `solve-report.json`, `explain.json`, `units.json`) and list them in bindings.md 1. In bindings.md 3.2 list Python methods for each (`obligations`, `ack`, `solve`, `emit`, `sync`, `import_`, `explain`), or state that these three groups are CLI-only in 0.1.0 and why. Add `--assume-unit` to `import` and `ir --observed`, and `import --accept` (writes bindings and scaffold; default is a dry-run proposal).

### 8. [gap] Adapter protocol cannot express `drive` as described; scope and matrix disagree
- adapters.md 1 says `drive` pushes, regenerates and "reads results back", but the protocol's `read` takes "file paths" only, `push` has no document target, and there is no `open`/`save`/`close` method; OQ-8 (unsaved session) cannot even be implemented. How a `ref`-mode dimension link is created in a live SOLIDWORKS session is not defined (`push` carries only path, value, locator).
- README scope says SOLIDWORKS adapter "read and drive"; the adapters.md 6 matrix gives SOLIDWORKS `emit` 0.1.0 too, and its mechanism ("COM API") duplicates `push`. If SOLIDWORKS emit means "write the .txt", that is the built-in `sw-equations`; if it means writing into the part, it is `push`.
- adapters.md 3a and README say "one format document per built-in emitter", but JSON and CSV (both 0.1.0 built-ins with `read`) have none; emitters/ holds only sw-equations.md.
- Fix: add protocol methods `open(files) -> session`, `read_live(session, locators)`, `save(session)`, and give `push` a `file` and `mode`; make OQ-8's recommendation normative here (refuse if `session.dirty`). Delete the SOLIDWORKS `emit` mark (README is right) or define it as "emit then link via sw-equations". Add `emitters/json.md` and `emitters/csv.md` (short: the header, order and precision rules of 3a) or narrow 3a to "each emitter with a native format".

### 9. [gap] Commercial and lifecycle data are magic parameter names, not a declared contract
- `unit_cost`, `lead_time`, `moq`, `supplier`, `mass` and `#[lifecycle]` drive roll-ups (language.md 3.5), `impact --swap` commercial deltas (obligations.md 5.1 item 7), `compare`, L001/L002 and the ScrewJoint cost story (stdlib.md 4), yet they are only "ordinary parameters" (language.md 3.1). Nothing checks spelling (`leadtime`), kind (Money, Time, Mass) or presence; `compare` has no definition of "commercial parameters"; the `Caster` interface in the main example declares none of them.
- Fix: add `std.commercial::Sourced` interface (`unit_cost: Money`, `lead_time: Time`, `moq: Count`, `supplier: Text`, `mass: Mass`, all optional via `?` roll-up semantics) and say in language.md 3.5 that the roll-up names and the swap/compare columns are exactly its members; a kind mismatch on those names is then ordinary I001/Q003. Optional: the lint `I005 commercial-member-wrong-kind`.

### 10. [gap] `import` is the adoption path but is specified in three scattered sentences
- adapters.md 3 and config-cli.md promise that `cadetta import` proposes canonical names, bindings and `.cdt` scaffolding, turns native expressions into `.cdt` references, and R006 fires on the first sync after import. Missing: how names are canonicalized (case canon N001, unit suffixes N003, `__`), grouping of several globals into components, how values with tolerance are recovered (SOLIDWORKS globals have none), what the proposal file looks like, how "first sync after import" is detected (R006 needs a stored state; the lock lists "last-sync digests" but not an adoption marker), and idempotence on re-import. The owner mainly uses SOLIDWORKS, where legacy files are the usual start.
- Fix: add `adapters.md 7 Import`: proposal = `proposal.toml` (bindings) + `proposal.cdt`; naming = split on `_`/case, strip unit suffix, one model per native file stem; native expression `"A" = "B" * 2` becomes `a = b * 2`; R006 state = lock key `adopted.<adapter>.<file>` written by `import --accept`, cleared by the first `sync`.

### 11. [gap] Determinism holes: severities and wall-clock inputs
- adapters.md 4.3 says a value not representable "returns Q004 ... never rounds silently", but lints.md makes Q004 a warning and unfixed: does emit still write a rounded value? As written, a warning plus a silently rounded file.
- L001 "becomes an error after" its date and W001 expiry depend on today's date, while obligations.md 2 claims the graph is "rebuildable from source plus the lock file at any commit". The same commit gives different exit codes on different days.
- Fix: make Q004 an error at emit time (value withheld, file not written for that path, `sync` exits 1); keep a warning only for `check`. Add `[project] as_of = "YYYY-MM-DD"` (default: today, `--as-of` flag); every date-dependent rule reads it, and `cadetta check --format json` records the date used in its header.

### 12. [polish] Open-question hygiene and small cross-reference defects
- OQ-10: language.md says "resolved in geometry.md", but geometry.md has no OQ-10 entry and the 1.3 decisions file lists "frames (OQ-10)" as still open. Pick one; if resolved, add a decision record and drop it from the decisions list.
- Status wording differs: "closed 2026-10-13" (OQ-1/3/5) vs "resolved in draft 1.3" (OQ-15) vs "resolved in geometry.md" (OQ-10). Use "closed DATE (ADR)" only, as README Conventions requires an ADR.
- Recommendations already assumed as decided elsewhere: OQ-16 (bindings.md 3.1 already describes Quantity with tolerance and the CLI parity rule; close it), OQ-7 (config-cli.md already pins `version` and adapters.md 5 says `initialize` returns the version; close, add "mismatch -> exit 2"), OQ-8 (implicit in drive; see finding 8). OQ-4 conflicts mildly with README's "CAD configurations out of scope": mark OQ-4 deferred past 0.1.0. OQ-13 backs a normative obligation row (finding 3).
- No global index: add an "Open questions" table in README (id, doc, status).
- Reserved keywords (language.md 2) omit `datum place rule key cols deps origin`; the item grammar has no productions for `waive`, `require`, `solver`, `fit`, `unit`, `kind`, `enum`, `fn`, `cite`, `interface`, or `type`. `#[native(adapter, "locator")]` (adapters.md 3) and `allow(CODE, "because")` (lints.md 2) are not in the attribute list (language.md 3.11, which shows `allow(CODE)`). `on` means three things (`Point2 on top`, `place .. on`, `fit .. on`) and `in` four (bounds, `for`, datum re-expression, `in` a datum type); add `in`/`satisfies` to the precedence list.
- README "Problems" table cites ranges that omit new codes: F001-F003 (F004), N001-N004 (N005, N006), R001-R006 (R007).
- obligations.md 3 "Nothing else writes state" conflicts with committed `<script>.out.json` (solvers.md 4) and `bindings/<adapter>.toml`; say "no other file under `.cadetta` and the lock".
- stdlib.md 3 table says `min_engagement` uses `Materials[into].engagement_factor`, but 4 says `FastenerPolicy.engagement` replaces it; write `policy.engagement[into].factor`. `impact --swap PART NEW` (CLI) vs `impact_swap(slot, alternate)` (Python): same names. `for alt NAME` is ambiguous when one alternate type is approved in two slots; use `for alt wheel=Colson2Series` as in the IR keys.

## Bolted-on / cost assessment (angle 5)
- Well integrated: tolerances/roles/fits, interface alternates with variant evaluation, obligations via closure digests, solvers (protocol is small; S003/S006 fit the graph).
- Weakly integrated: `process` compensation (FDM-driven, adds `designed`/`drawn`, P001, roles, `#[hole]`; finding 2 is its fallout; the owner uses SOLIDWORKS, so consider moving the feature to 0.2.0 if finding 2 cannot be fixed cheaply). Geometry/datums (finding 5) are the largest 0.1.0 surface with the thinnest CAD link. The optional frob section (obligations.md 6) is outside the product and can move to a decision record.
- Not worth its cost in 0.1.0 as specified: the `Vendor` tap mode and heat-set insert recipe in ScrewJoint (stdlib.md 3) while inserts are 0.2.0; the Python `cadetta.adapter` SDK plus pydantic models plus solver decorator are three Python deliverables that all gate the SOLIDWORKS adapter and the CI reference, so sequence them (adapter SDK first).
