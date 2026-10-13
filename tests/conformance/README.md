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
| `kind` | `"source"`, `"ir"`, `"binding"` | Fixture form: `.cdt`, IR JSON, or binding-call JSON. |
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
- Order does not matter; duplicates count (two expected, two produced).
- `must_fire.diagnostics` must contain the `rule` code (when `rule` is a
  diagnostic code). `must_not_fire.diagnostics` must not contain it.
- A rule that is not a diagnostic (a pure evaluation rule) uses `rule`
  as a name and lists `values` instead: `{ "path", "nominal" }` with
  quantity source literals, compared as exact quantities.

## Runner contract

The runner is `crates/cadetta-cli/tests/conformance.rs` (run with
`cargo test -p cadetta --test conformance`). It takes each
`NAME.expect.json`, runs the `entry` command on each fixture through the
`cadetta` library, and compares the result with the matching block.

- Single `.cdt` fixtures are staged as the package `acme.demo`
  (`src/lib.cdt`, `fail_on = "error"`); a `NAME.must-fire/` directory is
  copied as a whole package.
- Only `kind = "source"` with `entry` `check` runs today; other kinds and
  entries are skipped with a logged note until their entry points exist.
- Failures name the fixture (`<doc>/<NAME>`), the doc and the missing or
  extra code.
- `known-gaps.toml` lists fixtures whose implementation lags the spec
  (`fixture`, `ticket`, `reason`). A listed fixture may fail; a listed
  fixture that passes is a failure (stale entry).
- Every spec rule must have a pair; a rule without one is a corpus gap.
