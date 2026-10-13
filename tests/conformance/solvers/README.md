# Solver conformance fixtures

Must-fire / must-not-fire pairs for `docs/spec/solvers.md` (draft 1.4).
Layout and the base `expect.json` keys are in `tests/conformance/README.md`.

## Entries

| `entry` | Fixture | What runs |
|---|---|---|
| `check` | single `.cdt` or package | `cadetta check`; compared on diagnostics. The runner executes these. |
| `solve` | package (solver script included) | `cadetta solve [args]` in the staged package. |
| `eval` | package with committed `*.out.json` | evaluation; compared on `values`. |

Packages with a result file carry real `*.out.json` files produced by
`cadetta solve` (digests are genuine), or hand-built ones where the
binary cannot yet produce them (part-instance variants).

## Extra expect.json keys (non-check entries)

`args` (extra command-line arguments), `exit`, `stdout_contains`,
`stderr_contains`, `output` (map of package path to `exists`, `contains`,
`excludes`, `unchanged`), `runs` (script invocations expected), `config_error`,
`results_order`, `message_contains`, `stale_output`, `why`, `note`.

## Conventions

- A "property" rule (for example, shared runs, never-writes) uses a
  must-not-fire that is the near-identical scenario where the rule does
  not apply.
- Module `a` is `src/a.cdt`; solver paths are canonical
  (`acme.demo::Frame.wall_sizing`).

## Runner and known gaps

The runner executes only `check` entries today; the `solve` and `eval`
entries are written but skipped (ticket ~6BCVN48). Check fixtures the
binary fails are listed in `tests/conformance/known-gaps.toml`:
solver outputs not resolving in check (~JET4TC0), S007/Q001 on `in`
forms (~CXE9G4M), kindless `out` (~YBFVYDC). `solve` fixtures that
currently fail against the binary: output validation (~QDVP8JQ), S005
previous-result wording (~CHSJYEH), `--variants` with part instances
(~BJF002X).
