# Adapters conformance fixtures

Fixtures for `docs/spec/adapters.md` (draft 1.4). Layout and the
`expect.json` format are in `../README.md`; this file lists what is
specific to this directory.

## Entries

Each `NAME.expect.json` names an `entry`. Only `check` entries run in
the runner today; every other entry is written but skipped with a note
(runner gap ticket ~6BCVN48).

| entry | exercises |
|---|---|
| `check` | the lint report (R008 info, U001) |
| `emit sw-equations` | emit with bindings: mangling, scope, pins, R012, R013, Q004 |
| `classify` | three-way classification (spec 2.1) |
| `sync` | sync sessions, failures, adoption (2.2 to 2.5, 4, 5) |
| `read` | read-only adapter queries and R002/R003/R011 |
| `protocol` | handshake, version pins, contract scenarios |
| `push-plan` | the push plan and its preview |
| `impact-swap` | alternate-part swap impact |
| `emit-twice`, `emit-read`, `emit-compare` | byte-stability and round trips |
| `import` | import scenarios (spec 7) |

Scenario inputs live in the `must-fire` and `must-not-fire` directories
or JSON files next to the expect file.

## Extra expect keys

Beyond `diagnostics` and `exit`, the blocks may carry descriptive keys
read by humans and by future runner support: `class`, `action`,
`message_contains`, `writes`, `output` (`contains`, `excludes`),
`config_error`, `names_path_or_key`, `why`. The runner ignores them.

## Rule names

Rules with no diagnostic code use a descriptive kebab-case name in
`rule` (for example `locator-validation`) and an empty `diagnostics`
list.

## Configuration errors

An invalid bindings file or locator is a configuration error: exit 2,
naming the path and key, no diagnostic code (adapters.md 3). The binary
currently reports R005 with exit 0 (ticket ~3F67WS1).

## Known gaps

`r008-check-adapter-unavailable` and `r008-no-path-search` are listed in
`../known-gaps.toml` (ticket ~W41EHE6).
