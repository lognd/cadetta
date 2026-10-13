# Review 2: obligation graph and diagnostics UX

Summary:
- Development 3/5: the model (Merkle closures, computed staleness, lock-only state) is sound and distinctive, but obligations, attestations and waivers have no concrete identity, record format or selector grammar, so a team cannot yet operate it.
- Diagnostics 2/5: codes and severities are rich, but no diagnostic shape, text format or editor path exists.
- Most important fix: add an "Obligation identity and records" section to obligations.md (what an obligation is, its id, instance vs definition level, the attestation and waiver record, the selector grammar) and one `cadetta.diag/1` schema in lints.md. Everything else hangs off those two.

Scenario notes (simulated): three engineers A (source role), B (tolerance_review), C (author). Commits: (1) replace the cited datasheet PDF, (2) change a fit class, (3) change a solver input, (4) swap wheel primary, (5) re-ack. Branches `a/datasheet` and `b/review` both add attestations then merge. CI runs `check` and `check --release`. Where the spec stops being able to answer is noted per finding.

## Blockers

### 1. Waivers, requires and attestations contradict each other, and waiver digests have no home
Docs: obligations.md 4 and 3; lints.md 2 and W003; language.md 6 and 3.2.
- obligations.md 4: "Attestations are the only way to waive a `require`." lints.md 2 and W003: "A `require` is never waived; it is attested", and a waiver of a require is an error. language.md 6: "cannot be waived without an attestation". language.md 3.2: a violated `require` makes an alternate non-viable "whatever is waived". It is never stated whether an attestation can turn a `violated` require into an accepted one (state `waived`) or only add sign-off to a `holds` one. A team hitting a failed customer requirement cannot tell whether `check --release` can ever go green.
- lints.md 2 says a waiver "binds the closure digests of its targets at waive time" (needed for W002), but the `waive` syntax has no digest field and obligations.md 3 lists what the lock holds without naming waivers. W002 cannot be computed as specified. The grammar in language.md 3 lists `waive` as an item but has no production for it; the only syntax is the lints.md example (whose `Rover.load_ok` path in language.md 3.2 also lacks a namespace).
Fix: add to obligations.md 4: "A `require` has exactly two outcomes: `holds` (plus any configured attestation) or `violated`. A violated require is never accepted; an attestation never overrides a verdict. Attestations are sign-off on a verdict, not waivers. `waived` is a state only for diagnostics suppressed by `waive`/`allow`." Fix language.md 6 and lints.md 2 to match, and state that W003 covers `waive` naming a require code. Store waiver digests in the lock (`waiver <id> <path> <closure>`), add the grammar production `waive = "waive" CODE "on" glob [ "for" "alt" IDENT ] "{" "because" STRING [ "expires" DATE ] "}"` to language.md 3, and say that on W002 the suppressed diagnostic reappears (the waiver stops suppressing until re-reviewed).

### 2. Attestation level (definition vs instance) is unspecified, so the headline swap invalidation may not work
Docs: ir.md 2 and 4; obligations.md 2 and 5.1 item 6; stdlib/language examples.
ir.md 2's example fit path is the definition path `acme.rover::Axle.seat.shaft`; ir.md 4 gives the swap-sensitive closure only to instance paths (`Rover/wheel[2].hub_bore`, "the part node's own digest" included for interface-typed slots). A definition-level `Axle.seat` closure sees `wheel` only as an abstract parameter, so a tolerance reviewer's sign-off on `p6 on wheel.hub_bore` would survive a wheel swap and `impact --swap` item 6 would list nothing. Also unstated: do `wheel[0..3]` give four obligations or one?
Fix: in obligations.md 1 add: "An obligation targets a definition path when its closure reads no component parameter or interface-typed slot, otherwise an instance path (`acme.rover::Rover/axle[0].seat`). Instances with identical argument closures are one obligation with a list of instance paths." State in 5.1 item 6 that the attestation closure is the instance closure.

### 3. No diagnostic shape or output format is defined, though three docs promise one
Docs: lints.md intro and 1; bindings.md 3.2 ("the diagnostic schema"); config-cli.md 2; solvers.md 3; adapters.md 5.
lints.md says every diagnostic carries code, path and span; bindings.md says Python models are generated from "the diagnostic schema"; no document defines it. There is no field list, no notes/related locations, no suggested fix (despite N006 "suggests the instance", C003 "suggests accessors", `impact --swap` "suggested edit"), no text rendering, no ordering, no JSON example. Solver diagnostics (`severity`, `message` only) and adapter diagnostics use unrelated informal shapes. Spans are required "for design IR" only: R, O, S003, W and drift diagnostics point at native locators, lock lines or nothing.
Fix: add lints.md section 1.1 "Diagnostic record, `cadetta.diag/1`":
```
{ "schema":"cadetta.diag/1", "code":"C001", "severity":"error",
  "path":"acme.rover::Rover.load_ok", "alt":"Colson2Series"|null,
  "message":"...", "location": {"kind":"source","file":"cad/rover.cdt","start":{"line":41,"col":3},"end":{...}}
                               | {"kind":"native","adapter":"sw","file":"x.sldprt","locator":"D1@Sketch1"}
                               | {"kind":"lock","line":17} | null,
  "notes":[{"message":"...","location":{...}}],
  "fixes":[{"title":"...","edits":[{"file":"...","range":{...},"text":"..."}]}],
  "waived":null|{"waiver":"<id>","because":"..."} }
```
Require that solver and adapter responses reuse this record, sorted by (file, line, code) for determinism. Text form below (finding 11).

## Gaps

### 4. Obligations have no enumeration rule, id or selector grammar
Docs: obligations.md 2, 4, 5; config-cli.md 1, 2.
`covers = ["fit:*", "check:*", "ots:*"]` and `ack SELECTOR` use a selector syntax defined nowhere. It is unclear whether `ots:*` yields one source obligation per `ots` component, per parameter, or per citation; whether `fit:*` means fits only or the path glob `fit` kind; and what `open` means at adoption (a 200-parameter `ots` library with `source` required for `check` is red from day one with 200 O002s).
Fix: define in obligations.md 4: an obligation id is `<kind>:<canonical path>` with kinds `ots` (one per `ots` component, covering all its cited parameters), `fit`, `check`, `require`, `solver`, `sync`, `inspection`; selectors are `<kind>:<glob>` with `*`/`**` over canonical paths, comma-separated, and `ack` accepts exactly this grammar (plus `--stale`, `--open`, `--dry-run`). Add `cadetta ack --baseline SELECTOR --role R` for adoption, recording reason `baseline` and surfaced by `obligations` as "baselined, not reviewed".

### 5. "Why is this stale?" and "what does my change invalidate?" are not answerable
Docs: obligations.md 2, 5; ir.md 4.
`why PATH` is defined as upstream derivation of a value, not of a stale obligation. `impact PATH` is hypothetical ("if PATH changes") and takes no working-tree diff, so an engineer cannot ask "I edited three files, what did I just invalidate?" before committing. Digests are one-way, and the lock stores only digest sets, so cadetta cannot name which upstream node changed, nor show the old value; this is what a reviewer needs to re-review quickly.
Fix: (a) add `cadetta why --stale OBLIGATION`: walks the closure, reports the first (nearest-to-root) node whose own digest differs from the one recorded, with old and new values. To make that possible the attestation record stores, per covered node, the own digests of its transitive deps (compact list, sorted) plus the verdict and nominal value at ack time. (b) Add `cadetta impact --since REV` (default `HEAD`, also `--worktree`): compile IR at REV and the working tree, list changed nodes, obligations that become stale, verdicts that flip, CAD files to re-sync. State that this is the same machinery as `impact --swap` (diff of two evaluations). (c) `obligations --stale` shows per row: role, last attester, age, nearest changed dep.

### 6. `cadetta.lock` merge behaviour, record format, identity and time are unspecified
Docs: obligations.md 3, 4.
"Sorted and line-oriented so merges are reviewable" is the whole spec. Branches A and B each append attestations: textually likely conflict-free, but after the merge the merged closure usually differs from both branches' digests, so both attestations silently go stale (correct, but the lock merged cleanly and CI turns red with no explanation). Two attestations for the same obligation and role with different digests have no tie rule. "who" has no source (git config? `--as`?), "when" timestamps cause pointless diffs, nothing says which engineer may attest as `tolerance_review`, the author may attest their own change, and last-sync digests per adapter and file will conflict on every merge between two SOLIDWORKS users. Solver result digests are also in two places (obligations.md 3 says lock; solvers.md 4 says the committed `<script>.out.json`).
Fix: specify one line per record: `attest <obligation-id> role=<r> by=<email> at=<date> digest=<closure-hex> witness=<hash> reason="..."`, key `(obligation-id, role)`, newest `at` wins on duplicate keys and the loser is dropped by `cadetta lock tidy`. Ship `.gitattributes` `cadetta.lock merge=cadetta` and a `cadetta lock merge` driver (union by key, never silently keep a stale digest). Add `cadetta lock verify` that CI runs, listing attestations the merge invalidated and why. `by` comes from `git config user.email` unless `--as`. Add optional `[obligations.roles.X] members = [...]` and `distinct_from_author = true`. Move last-sync digests to a gitignored `.cadetta/sync.json` (they are per-machine state) and make solvers.md 4 the sole home of solver result digests; delete the claim from obligations.md 3.

### 7. Source attestations only work for local-file citations
Docs: language.md 3.9; obligations.md 2; lints.md S002, S004.
The example cites URLs (`cite colson = "https://..."`). "Value matches its citation" is stale only when "the cited local file's digest changed", so for URL, literal and `@measured` citations the source role is a one-time ack that can never go stale; the user's "datasheet changed" scenario works only if the PDF is vendored in the repo. There is also no per-locator granularity: replacing a 12-page datasheet makes every `ots` value citing it stale.
Fix: say it plainly in language.md 3.9: only `file` citations are drift-detected; URL citations are acknowledged at the text level. Add `cadetta cite pin NAME` that downloads the URL into `cites/` (content-addressed, committed or LFS) and rewrites the `cite` to the local path, and recommend it in S002/S004 text. Optionally allow a `pages` digest list later; for 0.1.0 whole-file staleness is acceptable if documented.

### 8. Obligation states, diagnostics codes and the CI story do not line up
Docs: obligations.md 2; lints.md O, S003, S004, R; config-cli.md.
The table has eight obligation kinds and six states, but the O family has three codes. "CAD file reflects design" (sync) and "as-built conforms" have no code; solver staleness is S003 and citation staleness is S004 while source attestation staleness is also O001, so a changed datasheet may fire both S004 and O001. Severity is set both by `[lint] severity` and by `required_for`, and the O001 default ("warning, error if required_for check") overrides the rule that only fixed codes are non-configurable. CI cannot run SOLIDWORKS (adapters.md 6), yet `check` must decide whether a stale `sync` obligation fails the build; the `roles` table can express only attestation roles, not sync/solver.
Fix: add to lints.md O: O004 `sync-stale` (info by default, since CI cannot read CAD), O005 `inspection-stale`; state that S003/S004 are the diagnostics for solver/citation staleness and O001 covers only attestation roles (no double firing); add `required_for` to `[obligations.kinds.<kind>]` so `sync` and `solver` can be made gating; document the severity precedence: `required_for` sets the floor, `[lint] severity` can only raise it. Add a mapping table "state x kind -> code" to obligations.md 2.

### 9. Waiver expiry and EOL make `check` non-deterministic
Docs: lints.md W001, L001; language.md 3.2.
W001 (error, fixed) and L001 ("error once its date has passed") depend on today's date, so the same commit can pass on Friday and fail Monday, and CI cannot reproduce a historic result ("graph rebuildable at any commit"). No lead-time warning exists, so a waiver expiring is a surprise red build.
Fix: add `--as-of DATE` (default today) to `check`/`obligations` and record the date in the check report; add `[lint] expiry_warn_days = 30` making W001 an info/warning before expiry (new code W005 `waiver-expiring`), and apply the same lead to L001.

### 10. Closure digests include presentation and process attributes, causing review churn
Docs: ir.md 4; language.md 3.11; obligations.md 2.
The own digest covers "attrs" (`emit`, `display`, `lifecycle`, `process`). Changing `#[display(in)]` or `emit(mid)` goes stale every tolerance review downstream, and a roll-up such as `mass?(self)` stales every `require mass_budget` attestation on any part's mass edit. With five commits a day reviewers will rubber-stamp, defeating the point.
Fix: split attrs into semantic (`process`, `role`, `hole`, `boss`; in digest) and presentational (`display`, `emit`, `allow`, `lifecycle` date text; excluded, with `emit` tracked separately in the sync digest) in ir.md 4. Record verdict and margin in the attestation (finding 5) and let `obligations --stale` print "verdict unchanged, margin 0.012mm -> 0.010mm", so re-review is cheap. Keep the stale state (do not auto-discharge).

### 11. No text format or editor path for diagnostics
Docs: config-cli.md 2; bindings.md; README scope.
Only `--format text|json` is named. `json` is the default when stdout is not a terminal, so CI logs are JSON blobs. No rendering is specified for text, and nothing says how a diagnostic reaches an editor. A GUI is out of scope, but an LSP is not a GUI and the span plus code design is exactly its input.
Fix: define text as `file:line:col: severity[CODE]: message` followed by an indented source excerpt, `note:` and `help:` lines (rustc style), with `path` printed under the excerpt; add `--format short` (one line, grep and problem-matcher friendly), `--format github` (workflow annotations) and `--format sarif` (code scanning). For editors: in 0.1.0, state that `cadetta check --format json --watch` emitting one `cadetta.diag/1` per line is the integration point for VS Code/Neovim problem matchers; record an ADR deferring `cadetta lsp` (diagnostics, hover = `why`, code lens = `impact`) as an explicit dropped/deferred ticket. Make the JSON default for non-TTY a flag-only behaviour (`CADETTA_FORMAT`) or keep text for CI by default.

## Polish

### 12. Smaller inconsistencies
- config-cli.md lists `check [--release]` only; `obligations`, `ack`, `lock verify` should take `--release` and `--role`, and `obligations` should have an exit code (1 if anything required for the chosen gate is stale) so CI can use it directly.
- obligations.md 5: `why.json`, `impact.json`, `compare.json` are named but `obligations`/`ack` output has no schema; add `obligations.json` and `ack.json`.
- lints.md 2 `#[allow(CODE)]` "is a waiver without expiry" but "still needs a `because`" and its example `#[allow(M001, "...")]` puts the reason as an argument; call this out in language.md 3.11 and say whether allow is W004-checkable and W002-bound (a waiver that never expires and never re-reviews contradicts W002).
- lints.md W001 and W004 have an empty Meaning cell.
- OQ-12 (swap events in the lock) is cheap once finding 6's record format exists; consider closing it with `swap` records so "why is this wheel here" survives squash merges.
