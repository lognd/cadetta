# The obligation graph

Core functionality. Every claim cadetta makes about a design (this value
came from that datasheet, this CAD file reflects this design, this check
holds, this reviewer signed off) is an obligation pinned to the digests of
what it depends on. When anything upstream changes, exactly the
obligations that depended on it become stale, and cadetta can say which,
why, and what discharges them.

## 1. Nodes and edges

Nodes are IR entities, each with a digest (ir.md 4): parameters, bounds,
citations, tables, fits, checks, requires, solvers and their result
files, parts and their alternates, bindings, native CAD files, and
attestations.

Edges:

| Edge | From -> to | Created by |
|---|---|---|
| `derives` | param -> each param its expression reads | the compiler |
| `cites` | param, table, component -> citation | `@` |
| `implements` | component -> interface | `: Iface` |
| `instantiates` | part -> primary and each alternate | `part` |
| `evaluates` | check, require, fit -> what it reads | the compiler |
| `solves` | solver output -> solver inputs, script, deps | `solver` |
| `binds` | param -> native locator | bindings |
| `realizes` | native file -> bound params | adapter `read` |
| `attests` | attestation -> a set of node digests | `cadetta ack` |
| `waives` | waiver -> target | `waive` |

## 2. Obligations

### 2.1 Identity and level

An obligation is one claim about one target, with an id (2.2) and a
set of covered nodes. Its level is fixed by what its closure reads:

- A **definition-level** obligation targets a definition path
  (`acme.rover::Axle.seat`) when its closure reads no component
  parameter and no interface-typed slot.
- Otherwise it is **instance-level** and targets an instance path
  (`acme.rover::Rover/axle[0].seat`). Its closure is the instance
  closure (ir.md 4), which includes the part node's own digest for
  interface-typed slots; this is what makes a swap invalidate it.
- Instances whose argument closures are identical are one obligation
  carrying the list of instance paths: `wheel[0..3]` with equal
  arguments is one obligation, not four. Instances that differ are
  separate obligations.
  The obligation id uses the first covered instance path; groups
  keep first-seen instance order.

### 2.2 Ids and selectors

An obligation id is `<kind>:<canonical path>`. Kinds:

| Kind | Target | One obligation per |
|---|---|---|
| `ots` | source sign-off of an `ots` component | `ots` component (all its cited parameters) |
| `fit` | sign-off of a fit's result | fit |
| `check` | sign-off of a check's result | check |
| `require` | sign-off of a `require` that holds | `require` |
| `solver` | solver result is current | solver |
| `citation` | a cited source is unchanged since acknowledged | `cite` item |
| `sync` | CAD file reflects design | native file bound by an adapter |
| `waiver` | a waiver was reviewed against its targets' current digests | `waive` declaration |

For `sync` the path is the repo-relative native file path. `cadetta check`
derives the state from the lock: when an adapter's current emitted value
differs from its last `synced` value, every file that adapter synced is
stale and reports O004.

A selector is `KIND:GLOB`, comma-separated for several:
`fit:acme.rover::Axle.*,check:acme.rover::**`. GLOB is the path glob
used by `waive` (lints.md 2): `*` matches within one path segment,
`**` across segments, `[*]` any index. `covers` in a role, `ack`,
`obligations` and `impact` all take exactly this grammar; a selector
that matches nothing is a usage error (exit 2).

Glob details (decided, 0.1.0): segments are separated by `::`, `.` and
`/`, so `*` never crosses any of them (`acme.rover::*` matches
`acme.rover::Axle` but not `acme.rover::Axle.press`). `[*]` matches any
index segment, `[N]`, `[A..B]` or a literal `[*]`. A selector with
several terms matches the union of its terms and is an error only when
the whole union is empty, not when a single term is. Empty terms,
missing `:`, an unknown KIND and an unterminated `[` are also usage
errors (exit 2).

### 2.3 Kinds, states and diagnostics

| Obligation | Discharged by | Stale when | Diagnostic |
|---|---|---|---|
| check / fit holds | evaluation, every run | never stale; recomputed | C001, C002, F004 |
| alternate is viable | variant evaluation (language.md 3.2): nothing violates | never stale; recomputed | I002, I003 |
| requirement is met | a `require` evaluating `holds` (verdict), plus its sign-off if configured | the require or anything it reads changed | C001; O003 for sign-off |
| solver result is current | `cadetta solve` | input, script or dep digest changed | S003 |
| CAD file reflects design | `cadetta sync` | a bound param's emitted value changed | O004 |
| value matches its citation | `cadetta ack ots:PATH --role source` | the citation's digest changed: a field (a new `rev`, `date` or `url`) or its `file`'s content | S004 |
| review / sign-off | (`cadetta obligations` ids) | Besides `check:`, `fit:`, `require:`, `ots:` and `cite:`, the list carries `solver:PATH` (judged against the solver's closure), `sync:FILE` (stale when a bound input changed) and `waiver:GLOB` (open until reviewed, stale when the covered closure digest changed). Check closures include the tables and part arguments a check reads, so editing them stales the attestation. |
| `cadetta ack SELECTOR --role NAME` | any node in the attested set changed digest | O001 (stale), O002 (open) |

Diagnostic rules:

- Citation staleness is S004 only; O001 is not raised for the same
  node. O001 and O002 cover role attestations alone.
- Solver staleness is S003; sync staleness is O004 (info by default,
  because CI cannot read CAD). A synced file whose adapters are all
  built-in without `drive` (`json`, `csv`, `sw-equations`) is an open
  `sync` obligation, also O004, with the reason "unverifiable without
  drive" (adapters.md 2); an external `drive` adapter that synced the
  same file discharges it, and so does an `attest` record on
  `sync:FILE`, which `cadetta ack sync:FILE --role R --reason "..."`
  records (offered for every file in a `synced file=` lock record;
  the role's `covers` must include `sync:FILE`). `cadetta sync` reports the open obligation itself (no
  separate `check` run needed).
- `[obligations.kinds.<kind>] required_for = "check" | "release"`
  makes `sync` and `solver` obligations gating, as for roles (4).
  Severity precedence is in lints.md 1.
- A citation goes stale when its record changes (language.md 3.9): a
  new `rev` or `date` is a new source, so the values cited to it need
  another look. Only a `file` is drift-detected on disk; a URL's
  content cannot be, and `cadetta cite pin NAME` downloads it into
  `cites/` (content-addressed) and adds the `file` field so it can.
  Whole-file granularity is the 0.1.0 behaviour.
- `cadetta ack ots:PATH --role source` records, per `cite` item the
  `ots` reaches, an `attest citation:<cite path>` with role `source`
  whose digest is the citation's digest (its type, fields and file
  digest). Any change makes the obligation stale and records S004. A
  changed locator is an edit of the citing member, so it stales the
  `ots:` attestation itself (O001), not the citation. Notes and
  pseudo-citations have no record and no check.
- The `sw-equations` adapter never discharges `sync`: the state stays
  `open` with reason "unverifiable without drive" (adapters.md).
- Decided (0.1.0): only `sync` and `solver` accept a kind-level
  `required_for`; any other kind is a configuration error. Only `stale`
  or `open` obligations gate (a `required_for = "release"` kind gates
  under `--release` only), and emission by `sw-equations` is never a
  discharge, so a gated `sync` obligation on an equations-only project
  fails `check --release` until a driving adapter syncs it.

The as-built obligation (inspection data read as observed IR, within
tolerance) is 0.2.0 and is not part of this table; see OQ-13.

States: `discharged`, `stale` (upstream changed since discharge),
`violated`, `undecided`, `open` (never discharged), `waived`. `waived`
applies only to diagnostics suppressed by `waive` or `allow` (4); no
obligation about a `require` is ever `waived`.

### 2.4 Staleness

Staleness is computed, never stored: an obligation records the closure
digests (ir.md 4) of the nodes it covers when it was discharged;
comparing them with the current graph gives the state. Because closure
digests are Merkle over `deps`, an attestation on a check goes stale
when anything the check reads changes, however far upstream. This makes
the graph rebuildable from source plus the lock file at any commit
(date-dependent rules read `--as-of`, 5).

Only semantic attributes enter digests (ir.md 4): process, role, rule
roles and default. Presentational attributes (display, emit, allow,
lifecycle date text) do not, so changing `#[display(in)]` or an `emit`
point does not stale a review. A changed `emit` point changes the
emitted value, and is tracked by the `sync` record (3) and so stales
the `sync` obligation only.

An attestation also stores its verdict, margin and the own digests of
the deps it covered (3), so that a stale obligation can be explained
(`why --stale`) and re-reviewed cheaply. A stale attestation is never
auto-discharged, even when the verdict is unchanged.

Decided (0.1.0), implemented in `cadetta-obligations` `staleness.rs`:

- The state of an attestation is a pure function of the record and the
  current graph: `open` without a record, `current` when the recorded
  closure digest equals the current one, `stale` otherwise. The reason
  for `why --stale` is the difference between the recorded and current
  sorted sets of dep own digests (added and removed); an empty
  difference means the covered node itself changed.
- `why --stale` (`why_stale.rs`) walks the current closure breadth-first
  from the covered nodes and reports the first dep, nearest the root,
  whose own digest is not among the recorded dep digests: its new value
  is its current own digest, its old value the recorded digest that
  vanished (exact when exactly one vanished, otherwise the vanished
  digests are listed). With no changed dep the covered node itself
  changed. The lock stores digests only, so value text for old and new
  comes from the CLI evaluating the IR at the attested revision.
- An obligation covering several instance paths of differing closures
  has one covered digest: a single closure as is, otherwise the sorted,
  de-duplicated closures hashed together, so any member changing stales
  it.
- A `require` that `holds` with a sign-off role configured is
  `discharged` only with a `current` attestation, `open` without one and
  `stale` with an old one; a `violated` or `undecided` require keeps its
  verdict whatever attestations exist (4).
- The as-built obligation (0.2.0, OQ-13) reuses the same machinery: an
  inspection is a recorded verdict at a closure digest; it is `stale`
  when the design changed after the inspection, otherwise it takes the
  inspection verdict.

## 3. The lock file

`cadetta.lock` is committed and holds everything that is not derivable
from source. One record per line, sorted by (kind, key), no
timestamps finer than a date, so merges are reviewable:

| Record | Form | Key |
|---|---|---|
| `attest` | `attest ID role=R by=EMAIL at=DATE digest=HEX verdict=V margin=Q deps=HEX,... reason="..."` | (ID, R) |
| `waiver` | `waiver CODE GLOB [alt=SLOT=ALT] digest=HEX` | (CODE, GLOB, alt) |
| `synced` | `synced ADAPTER PATH locator=L value=V` | (ADAPTER, PATH, L) |
| `synced` (file) | `synced ADAPTER file=FILE digest=HEX` | (ADAPTER, FILE) |
| `adopted` | `adopted ADAPTER FILE` | (ADAPTER, FILE) |
| `package` | `package NAME version=V source=S digest=HEX` | NAME |
| `swap` | `swap SLOT from=A to=B by=EMAIL at=DATE reason="..."` | whole line |

- `attest` stores who, role, date, the closure digest of the covered
  set, the verdict and margin at ack time, and the sorted own digests of
  every transitive dep (the basis of `why --stale`). Attestations of a
  baseline (`ack --baseline`) carry `reason="baseline"`.
- `waiver` stores the closure digest of the waiver's targets at review
  time; lints.md 2 uses it for W002.
- `synced` is the base for adapter three-way sync (adapters.md): the
  last emitted value per locator, and the saved file digest per native
  file. It is committed, not ignored: it describes committed CAD files,
  and two branches syncing one locator differently are a real conflict.
- `adopted` marks an imported file until its first sync (R006).
- `package` pins one resolved dependency: version, source and the digest
  of its content (packages.md 1). The digest enters the closure digest
  of everything that reads the package, and `cadetta update` rewrites
  the record. Obligations across packages (upstream attestations,
  `trust`) are defined in packages.md 6.
- `swap` records a part swap (who, why, which alternate) so "why is this
  wheel here" survives squash merges. `cadetta swap SLOT NEW --reason
  "..."` applies the suggested edit of `impact --swap` (5.1) and appends
  the record. Swap records are an audit trail: `lock tidy` keeps them.

Solver results are not in the lock. Their digests live in the
committed `<stem>.out.json` (solvers.md 4), the only home of that
state.

Text format (normative for 0.1.0):

- A record is one line ending in `\n`; the file has no blank lines,
  comments or header, and an empty lock is an empty file. Records are
  ordered by kind word bytewise (`adopted`, `attest`, `package`, `swap`,
  `synced`, `waiver`), then by key fields in table order; `synced`
  value records sort before `synced` file records of the same adapter.
  A `swap` key is its whole rendered line.
- Fields are separated by single spaces. A word is bare unless it is
  empty or has whitespace, control characters, `"` or `\`, or (when
  positional) `=`; then it is double-quoted with escapes `\\`, `\"`,
  `\n`, `\r`, `\t`, `\u{HEX}`. `reason` is always quoted.
- `at` is `YYYY-MM-DD`; digests are lowercase hex; `verdict` is
  `holds`, `violated` or `undecided`; `margin` is the quantity text;
  `deps` is the sorted, unique, comma-joined list (empty allowed).
- Citation acknowledgements are `attest citation:PATH` records, PATH
  being the `cite` item's canonical path; there is no separate citation
  record. Loading tolerates unsorted or
  duplicate lines (the tie rule applies); `lock verify` flags any file
  that is not byte-identical to its canonical rendering.
- Writing is atomic (temp file then rename) and skipped when the bytes
  are unchanged.

Rules:

- **Identity.** `by` is `git config user.email` unless `--as EMAIL`.
  `at` is a date (`--as-of` applies).
- **Tie rule.** On duplicate keys the newest `at` wins; on equal `at`
  the bytewise greater line wins. The loser is dropped by `lock tidy`.
- **Merge driver.** `.gitattributes` carries `cadetta.lock
  merge=cadetta`; `cadetta lock merge BASE OURS THEIRS` unions by key
  with the tie rule and never keeps a record whose digest it cannot
  validate. Two different `synced` values for one key are left as a
  conflict, never resolved silently.
- **`cadetta lock verify`** (run by CI) checks the lock is well formed
  and sorted, keys are unique, every `attest` names an existing
  obligation and role and satisfies the role's `members` and
  `distinct_from_author` rules (4), and lists the attestations a merge
  invalidated and why. Exit 1 on any finding. `--format json` emits the
  `cadetta.lock/1` report (`docs/schemas/lock.json`): `clean`,
  `exit_code` and one `findings` entry per problem, each with a stable
  `kind`, a `message` and the fields of that kind.
- **`cadetta lock tidy`** drops superseded duplicates and records whose
  obligation no longer exists, and re-sorts. It never drops `swap`.

Besides the lock, cadetta writes only declared artifacts, none of which
carries obligation state of its own: `<stem>.out.json` (solvers.md 4),
`bindings/<id>.toml` (adapters.md), `docs/schemas/` and `build/`
(config-cli.md lists them all). Nothing else writes state.

## 4. Roles, policy and the `require` rule

`[obligations]` in `cadetta.toml` declares roles and what they must
attest:

```toml
[obligations.roles.tolerance_review]
covers = ["fit:*", "check:*"]
required_for = "release"
members = ["ana@acme.example", "bo@acme.example"]
distinct_from_author = true

[obligations.roles.source]
covers = ["ots:*"]
required_for = "check"

[obligations.kinds.sync]
required_for = "release"
```

`members` (optional) restricts who may attest the role; an attestation
by anyone else is rejected by `ack` and reported by `lock verify`.
`distinct_from_author = true` requires `by` to differ from the author of
the newest commit that changed any covered node.

`cadetta check` fails on stale or open obligations whose role or kind is
`required_for = "check"`; `cadetta check --release` also fails on the
`release` ones. `required_for` sets a severity floor (error); see
lints.md 1 for precedence with `[lint] severity`.

Decided (0.1.0): a role gates an obligation when one of its `covers`
selectors matches the id and the obligation is `stale` or `open`; the
first matching role with a gating `required_for` is reported, and a
kind-level `required_for` (2.3) gates independently. `ack` rejects an
attestation whose role does not cover the obligation, whose `by` is not
in `members`, or (with `distinct_from_author`) equals the author; an
unknown author fails closed. A `require` can only be signed off by a
current attestation; a waiver request without one is refused.

Adoption: `cadetta ack --baseline SELECTOR --role R` discharges every
matching open obligation at current digests with reason `baseline`, so
a large existing library does not start red. `obligations` lists them
as "baselined, not reviewed".

**The `require` rule** (the single statement; language.md 6, lints.md 2,
5.1 and language.md 3.2 refer here):

> A `require` is never suppressed. Its verdict is evaluated like any
> check. An attestation is sign-off on a verdict, never a waiver: it
> discharges the sign-off obligation of a `holds` require (O003) and
> cannot change a `violated` one, which stays C001. The state `waived`
> applies only to diagnostics suppressed by `waive` or `allow`.

A `waive` or `allow` that names a diagnostic raised by a `require` is
W003.

## 5. Queries

| Command | Answers |
|---|---|
| `cadetta why PATH` | Upstream: the derivation chain from a value back to citations, tables, solvers and requirements. |
| `cadetta why --stale OBLIGATION` | Why an obligation is stale: walks its closure and reports the first node, nearest the root, whose own digest differs from the recorded one, with old and new values. |
| `cadetta impact PATH` | Downstream: everything that goes stale or changes verdict if PATH changes. |
| `cadetta impact --since REV` / `--worktree` | What a change invalidates: compiles the IR at REV (default `HEAD`) and at the working tree, then lists changed nodes, obligations that become stale, verdicts that flip, and CAD files to re-sync. The same machinery as `--swap`: a diff of two evaluations. |
| `cadetta impact --swap PART NEW` | A swap preview (5.1). |
| `cadetta compare IFACE` | Every implementer of an interface side by side: interface members, bounds, commercial members, lifecycle, and which checks hold for each in this design. |
| `cadetta obligations [--stale] [--role R] [--release] [--as-of DATE]` | The open and stale obligation list: the design's to-do list. Exit 1 if any obligation required for the chosen gate is stale or open. |
| `cadetta ack SELECTOR --role R --reason "..." [--release] [--baseline] [--dry-run] [--as EMAIL]` | Discharge attestation obligations at current digests. |
| `cadetta ack waiver:GLOB --role R` | Record (or re-review) the closure digest of each matching waiver in the lock (lints.md 2); no `--reason` is needed. Every `waive` whose glob matches at least one node is offered, so it is reviewable as listed by `obligations`. `sync:FILE` has no ack: it is discharged by `cadetta sync`. |
| `cadetta lock verify` / `tidy` / `merge` | Lock maintenance (3). |
| `cadetta cite pin NAME` | Vendor a URL citation as a local file (2.3). |

`why PATH` walks the IR from PATH upstream (schema `why.json`,
`cadetta.why/1`). A member reads its `deps`, its citation (a leaf of kind
`citation`) and, when a solver outputs it, that solver; a solver reads its
`deps` and inputs; a check reads every member whose path occurs in its
expression. Children are sorted by path (citation first); a node reached a
second time is listed with `repeated = true` and not expanded again. A path
the IR does not define is a usage error (exit 2); an undefined dependency
(a table, say) appears as an `unresolved` leaf.

`impact PATH` walks the IR from PATH downstream (schema `impact.json`,
`cadetta.impact/1`): the reverse of the reads `why` follows. A member
reading PATH `changes`; a solver reading it (deps or inputs) goes `stale`;
a check whose expression mentions a reached member `may_flip`. Each
dependent carries `via` (the node it reads that leads to PATH) and `depth`;
the list is breadth first, then sorted by path. `obligations` lists
`solver:PATH` (stale) and `check:PATH` (may_flip) ids. Whether a verdict
actually flips needs two evaluations (`--since`, `--swap`). A path with no
dependents prints `PATH: no dependents` (JSON: empty arrays); an undefined
path is a usage error (exit 2).

`impact --since REV` and `impact --worktree` diff two evaluations: `--since
REV` compiles the project as of git REV (via `git archive`, no checkout is
touched) against the working tree; `--worktree` alone compares `HEAD` with
the working tree. The report (schema `impact-since.json`, id
`cadetta.impact-since/1`) has four sections: changed nodes (members,
solvers, checks; added, removed or modified), obligations affected (a
changed solver or its dependents `stale`, a changed check `may_flip`, a
check whose verdict differs `flipped`), verdicts flipped (old and new), and
CAD files to re-sync (bindings whose member changed or depends on a
changed member). An empty diff prints `no changes`.

`obligations --stale` prints per row: obligation id, role, last
attester, age, nearest changed dep, and when the verdict is unchanged
"verdict unchanged, margin 0.012mm -> 0.010mm", so re-review is cheap.

`--as-of DATE` (check and obligations; default today; also `[package]
as_of`) fixes the date read by every date-dependent rule (W001, W005,
L001, attestation age). The JSON report header records the date used,
so a historic result is reproducible.

Resolution order: `--as-of`, then `[package] as_of`, then the system
date (UTC). The date is `YYYY-MM-DD`, validated strictly (impossible
days such as `2026-02-30` are a usage error, exit 2). An expiry has
passed only when its date is strictly before the as-of date, so a
waiver with `expires = as_of` still suppresses on that day. The report
header carries `as_of` and `as_of_source` (`flag`, `config` or
`today`). Implemented in `cadetta-obligations::as_of`.

All queries emit JSON (`--format json`) so other tools, including frob,
can consume them. Their schemas live in `docs/schemas/` and are
generated from the Rust types: `diag.json` (the `cadetta.diag/1`
record, lints.md 1.1), `check.json`, `impact.json`, `swap.json`,
`compare.json`, `why.json`, `obligations.json` (the
`cadetta.obligations/1` report, which carries obligation state; state
is not part of the IR), `ack.json`, `lock.json` (the `lock verify` report) and `sync.json`. config-cli.md
holds the full command table.

### 5.1 `impact --swap` semantics

The preview is the variant evaluation of `PART=NEW` (language.md 3.2),
diffed against the primary evaluation. `NEW` must already implement the
part's interface; it need not be in the `alt` list (then the output says
"not yet approved"). Output sections, in this order, each sorted by
canonical path:

1. **verdict**: `ok`, `ok with warnings` (undecided results), or
   `blocked` (a violation; a violated `require` stays violated, 4).
2. **values that change**: direct (interface members) then derived,
   with old and new values; a `let` is listed like any value.
3. **verdicts that flip**: checks, fits, requires, bounds.
4. **suggested edit**: the new `part` line, `NEW` removed from `alt`
   and the old primary appended.
5. **CAD files to re-sync**: files bound to any path whose emitted value
   changes, with the paths; files with no changed bound value are
   listed as not stale. For an assembly file the action to replace the
   component is listed explicitly ("replace wheel<2> with
   Colson2Series.SLDPRT in Rover.SLDASM"); bindings keyed `for =
   "<alternate>"` (adapters.md) become the active locator set, and R011
   reports an assembly still holding the old component.
6. **attestations invalidated**: each attestation whose covered
   instance closure digests would change, with the count of covered
   nodes affected.
7. **commercial deltas**: the diffs of `sum?(self, unit_cost)` per
   currency, `max?(self, lead_time)` and `mass?(self)`, the same roll-ups
   the language defines (language.md 3.5, every instance counted), at
   the assembly named by `PART`'s prefix; `moq` per changed part. The
   names are the members of `std::commercial::Sourced` (stdlib.md); a
   wrong kind there is an ordinary I001 or Q003. Parts lacking a value
   are listed. A design `let` over the same roll-up therefore always
   agrees with the preview.

In text output numbers print by the display rules (quantities.md 6),
money in the currency's minor unit and percentages to one decimal, half
away from zero. JSON carries exact values as source literals, never
rounded. `swap.json` has a `swap` header (slot, alternate, approved or not) and
one key per section, in order: `verdict`, `values_changed`,
`verdicts_flipped`, `suggested_edit`, `cad_resync`,
`attestations_invalidated`, `commercial_deltas`. The schemas (above) are generated with the implementation;
until then this section is normative.

Implementation rules for sections 5 to 7 (decided, frob ticket C8VKPXW):

- **File of a locator**: the text after its last `@` when that has a file
  extension (`Rover.SLDPRT`); otherwise the adapter's globals, printed
  `(<adapter> globals)`.
- **Stale**: a bound path is stale when it is in the values-changed set,
  or it has a locator `for = "NEW"`. A `for` locator for another
  alternate is inactive and never makes a file stale. Replace actions
  come from locators carrying a `component` instance, and name the first
  `for = "NEW"` locator without a `component` as the new file.
- **Attestations**: an attestation on a check is listed when its covered
  closure (the check and every transitive dependency) meets the changed
  values, the flipped checks and the slot itself; the count is the size of
  that intersection.
- **Roll-ups** walk the parts of the assembly with `NEW` replacing the
  slot's primary, multiplying counts through sub-assemblies. A component
  that declares the member supplies it (its variant value for the swap
  when present) and stops the descent; one that lacks it but has parts is
  descended into; otherwise the part is listed as lacking a value. Inexact
  terms are skipped with a warning. Mass prints in kg, lead time in days.
  The percent change is shown only when the old value is non-zero.

`compare IFACE` is the same evaluation for every implementer of the
interface in every slot typed by it, as a table. Its commercial columns
are the members of `std::commercial::Sourced`.

## 6. frob integration (optional)

Cadetta MAY act as a frob sibling (like grimble): `frob check` would run
`cadetta check` and gate on its stale obligations, and frob tickets could
bind to canonical paths (`acme.rover::Axle.end_diameter`). Cadetta never
requires frob.

## Open questions

- OQ-12: closed 2026-10-13 (obligations.md 3): part swaps are `swap`
  records in the lock file, written by `cadetta swap`.
- OQ-13: Inspection data formats (CMM CSV, QIF), how measured features
  bind to canonical paths, and the "as-built conforms" obligation
  (0.2.0).
