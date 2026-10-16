# Conformance corpus

One must-fire and one must-not-fire fixture for every normative rule
and diagnostic code of the spec (`docs/spec/README.md`), so any
implementation can be tested against the spec alone. The golden corpus
(`tests/golden/`) is a different thing: realistic whole designs with
hand-evaluated results.

## Layout

```
tests/conformance/<doc>/
  NAME.must-fire.cdt         source that must produce the expected codes
  NAME.must-not-fire.cdt     near-identical source that must not
  NAME.expect.json           expectations for both fixtures of NAME
  NAME.must-fire.json        IR or binding fixture, when the rule is
  NAME.must-not-fire.json      not source-level (replaces the .cdt pair)
```

- `<doc>` is the spec document stem: `language`, `quantities`, `lints`,
  `obligations`, `packages`, `ir`, `solvers`, `adapters`, `bindings`,
  `config-cli`, `stdlib`, `geometry`.
- `NAME` is the rule or diagnostic code in lowercase, optionally with a
  short kebab-case qualifier when a code needs several cases:
  `q006.must-fire.cdt`, `c002-nominal.must-fire.cdt`. Names are unique
  within a `<doc>` directory.
- A fixture pair shares one `NAME` and one `NAME.expect.json`. The two
  fixtures should differ by the smallest edit that flips the rule.
- Multi-file fixtures use a directory `NAME.must-fire/` (and
  `NAME.must-not-fire/`) holding a package: `cadetta.toml` and `src/`
  (packages.md 1, 2). The `expect.json` sits beside the directories.
- Fixture files are ASCII only.

## `expect.json`

```json
{
  "spec": "1.4",
  "doc": "quantities",
  "section": "quantities.md 1",
  "rule": "Q006",
  "kind": "source",
  "entry": "q006",
  "must_fire": {
    "diagnostics": [
      { "code": "Q006", "severity": "warning", "path": "acme.demo::Part.width" }
    ]
  },
  "must_not_fire": {
    "diagnostics": []
  }
}
```

| Field | Type | Meaning |
|---|---|---|
| `spec` | string | Spec draft the fixture is written against. |
| `doc` | string | Spec document stem; equals the directory name. |
| `section` | string | Normative section the rule comes from. |
| `rule` | string | Rule or diagnostic code the pair tests. |
| `kind` | `"source"`, `"ir"`, `"binding"`, `"compare"` | Fixture form: `.cdt`, IR JSON, binding-call JSON (see "Binding fixtures"), or an input pair (see "Two-input comparisons"). |
| `entry` | string, optional | Command to run: `check` (default), `eval`, `lint`, `emit <target>`. |
| `must_fire` | object | Expectations for the must-fire fixture. |
| `must_not_fire` | object | Expectations for the must-not-fire fixture. |

Each of `must_fire` and `must_not_fire` holds:

- `diagnostics`: list of `{ "code", "severity", "path"? }`.
  `severity` is `error`, `warning` or `info`. `path` uses the full
  canonical form (ir.md 2) and is optional; when present it must match.
- `exit`: optional integer, the expected process exit status. Default:
  nonzero if any error is expected, else 0.

Matching rules:

- The `diagnostics` list is closed for errors and warnings: an error or
  warning the run produces that the list lacks is a failure, as is a
  listed one the run does not produce. Info diagnostics are compared
  only when listed.
- U002 (unused private item) is the exception: fixtures declare private
  components nothing instantiates, so the runner drops U002 from a run
  unless the fixture's `expect.json` lists U002 in either block.
- Order does not matter; duplicates count (two expected, two produced).
- `must_fire.diagnostics` must contain the `rule` code (when `rule` is a
  diagnostic code). `must_not_fire.diagnostics` must not contain it.
- A rule that is not a diagnostic (a pure evaluation rule) uses `rule`
  as a name and lists `values` instead: `{ "path", "nominal" }` with
  quantity source literals, compared as exact quantities.

## Binding fixtures (`kind: "binding"`)

Copied from `bindings/README.md` ("Entry and fixture form"). Binding
pairs are `kind = "binding"`, `entry = "python"` and describe Python-API
behaviour; the runner skips them except `classify` (see "Runner
contract"). They are replayed by the Python binding suite,
`bindings/python/tests/test_binding_fixtures.py` (ticket ~CDRJTN9).

A fixture is one API call:

| key | meaning |
|---|---|
| `api` | dotted callable under `cadetta` (`Project.open`, `Quantity.__add__`, `cadetta.quantity`) |
| `args` | positional arguments; quantity arguments are source literals parsed with `cadetta.quantity` |
| `files` | optional package tree (relative path to contents) staged as the project directory |
| other keys | call context (`role`, `edit`, `inputs`, `outputs`, `returns`, ...) |

Expectation blocks use `result` (`Ok`, `Err`, `raises`, `value`,
`protocol-error`) and descriptive keys (`QuantityError` = expected
`.code`, `error`, `value`, `nominal`, `package`, `modules`, ...).
`exit` is the CLI exit code the call maps to (0 clean, 1 error
diagnostics, 2 I/O, configuration or usage).

## Two-input comparisons (`kind: "compare"`)

The digest rules (ir.md 4: formatting never changes a digest, semantic
versus presentational attributes, closure Merkle, swap staleness) and
schema `classify(old, new)` (ir.md 5) relate two inputs, which the
single-fixture contract above cannot express. A `compare` fixture is a
pair of inputs per side; the verdict is a relation between them.

```
tests/conformance/ir/
  NAME.expect.json
  NAME.must-fire.a.cdt         input A of the must-fire side
  NAME.must-fire.b.cdt         input B (the edited one)
  NAME.must-not-fire.a.cdt
  NAME.must-not-fire.b.cdt
```

`a` and `b` use the extension of the input form: `.cdt` (or a
`NAME.must-fire.a/` package directory) for digest rules, `.json` for
`classify`, where `a` is the old and `b` the new schema document.

```json
{
  "spec": "1.4",
  "doc": "ir",
  "section": "ir.md 4",
  "rule": "digest-presentational-attrs",
  "kind": "compare",
  "entry": "digest",
  "must_fire": {
    "digests": [
      { "path": "acme.demo::Part.width", "field": "closure", "relation": "differs" }
    ]
  },
  "must_not_fire": {
    "digests": [
      { "path": "acme.demo::Part.width", "field": "closure", "relation": "same" }
    ]
  }
}
```

- `entry` is `digest` (compile A and B, compare node digests) or
  `classify` (compare two IR JSON Schemas).
- `digests` lists `{ "path", "field", "relation" }`: `field` is `own` or
  `closure` (ir.md 4), `path` is a canonical node or instance path
  present in both inputs, and `relation` is `same` or `differs`. Paths
  not listed are not compared.
- For `classify`, a block holds `"classify": "minor"`, `"major"` or
  `"none"`, the result of `classify(a, b)` by the ir.md 5 rule (a new
  optional property or definition is `minor`; a removed or retyped
  property, a new required property, a changed enum or a removed
  definition is `major`; identical documents are `none`).
- "Fires" means the rule's observable effect: a digest that must change
  (`differs`) belongs in `must_fire`, one that must stay put (`same`) in
  `must_not_fire`; a swap-staleness pair therefore lists the swapped
  slot's instance paths as `differs` in `must_fire` and an unrelated
  check as `same` in `must_not_fire`. The two sides use the same `a`
  and differ only in `b`, by the smallest edit that flips the rule.
- `diagnostics` is not used by `compare`; an input that fails to compile
  is a failure of the fixture, reported as such.
- The executor lives in `crates/cadetta-cli/tests/conformance/two_input.rs`:
  `digest` stages both sides, reads `cadetta ir` for each and reports the
  observed `relation` per row; `classify` calls `cadetta_ir::schema::classify`
  (`Breaking` is spelled `major`). A fixture for a rule the implementation does
  not yet honour fails like any other and is listed in `known-gaps.toml`.

## Observing the emitted point

The point a parameter is written at (quantities.md 5.6: `nominal`, `mid`,
`mmc`, `lmc`, from `[emit] point`, `#[emit(..)]` and `#![emit(..)]`,
inherited through pure references) is observed through a package fixture
(`NAME.must-fire/` with a `json` adapter, `bindings/json.toml` and
`"entry": "emit json"`), not through `eval`, which reports designed
values. The `output` expectation names `build/params.json` and lists the
written `"value": "..."` text under `contains`/`excludes`; the json
adapter writes the value in the locator's `native_unit`. The `quantities`
`emit-point-*` fixtures follow this form; the emitters' own `note-*`
fixtures observe the same point through the note text.

## Runner contract

The runner is `crates/cadetta-cli/tests/conformance.rs` (run with
`cargo test -p cadetta --test conformance`). It takes each
`NAME.expect.json`, runs the `entry` command on each fixture through the
`cadetta` library, and compares the result with the matching block.

- Single `.cdt` fixtures are staged as the package `acme.demo`
  (`src/lib.cdt`, `fail_on = "error"`); a `NAME.must-fire/` directory is
  copied as a whole package.
- Doc directories may nest (`emitters/sw-equations`); the doc name is the
  path below `tests/conformance`, and `expect.json` `doc` must equal it.
- Every `(kind, entry verb)` pair is dispatched through one table in
  `crates/cadetta-cli/tests/conformance/exec.rs`. Executors today:
  `source` with `check`, `config` (runs `check`), `ack`, `emit`, `solve`,
  `fmt`, `names`, `bind`, `cite`, `eval` (in-process IR values) and `init`; `compare` with
  `digest` and `classify` (two inputs, `two_input.rs`); and
  `binding` with `classify` (sync dry-run) and `protocol` (the adapter host
  JSON-RPC over a scripted in-memory adapter; `protocol.rs` records `sent`,
  `refused`, `request`, `protocol_error` and the like), `read` and `emit-read`
  (`read.rs`: the adapter host's read side in process, namely native-link
  discovery, R003, name qualification, unit resolution, the csv, json and
  sw-equations readers and the round-trip contract; the `observed` block is
  subset-compared and `excludes` lists tokens that must be absent) and
  `push-plan` (`push_plan.rs`: the tolerance planner, one printed line per
  written tolerance, checked by a flat `output` `contains`/`excludes` against
  that plan text; geometry push-plan fixtures remain gaps) and `sync`
  (`sync.rs`: the adapter host's sync loop against a simulated native
  document, observing `class`, `writes`, `regen`, `lock_updated`,
  `lock_synced_updated`, `push_status` and the next run's class; fixtures
  needing a tool session are gaps) and `import` (`import.rs`: propose mode
  only; observed items are decoded as the CLI does and run through
  `propose`, observing `members`, `components`, `flagged`, `item_order`,
  `status` and `reason_present`; accept, update and richer item features
  are gaps and fail loudly). A pair with no executor is a
  failure ("no executor for kind K entry V"), never a silent skip; the
  fixture must then be listed in `known-gaps.toml` against the ticket that
  adds the executor.
- A block may carry `args`: extra CLI arguments appended after the entry
  tokens. `emit TOKEN` names an adapter; the runner maps the name to the
  `id` in `cadetta.toml` when one is set.
- Beyond `diagnostics`, `exit` and `values`, the runner observes `output`
  and `outputs` (`contains`/`excludes` per file), `writes`,
  `file_written`, `order`, `stdout_contains`, `stderr_contains`,
  `message_contains`, `message_excludes`, `config_error` (exit 2),
  `json_diag_schema` (every report record carries the schema tag),
  `results_order` (solver ids appear in this order in the written files),
  `runs` (lines in the `marker` file each solver script appends to),
  `unbound_params_not_written` (no written file names a member that no
  `bindings/*.toml` binds) and, for `classify`, `r001_suppressed_by_force`
  (`force_design` overrode an R001 that would have refused; the override
  note is then not counted as a diagnostic).
  `note`, `why` and `args` are descriptive and ignored. Any other key is
  a failure ("this entry does not observe K") so an expectation is never
  dropped silently.
- Failures name the fixture (`<doc>/<NAME>`), the doc and the missing or
  extra code.
- `known-gaps.toml` lists fixtures whose implementation lags the spec
  (`fixture`, `ticket`, `reason`). A listed fixture may fail; a listed
  fixture that passes is a failure (stale entry).
- Every spec rule must have a pair; a rule without one is a corpus gap.
- The runner's test threads share one process: fixture files are written
  under `run::writing()` and the CLI (which may spawn adapters) runs under
  `run::spawning()`, so a spawn never inherits an open script handle
  (ETXTBSY, flaky under load).
