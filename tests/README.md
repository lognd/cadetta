# Tests

| Directory | What it holds |
|---|---|
| `golden/` | Realistic designs with their expected results, one package per case (`golden/README.md`) |
| `conformance/<doc>/` | One must-fire and one must-not-fire fixture per rule and diagnostic code of each spec document; diagnostic fixtures live in `conformance/lints/<code>/`. Built by the conformance-corpus tickets. |

Unit tests live with the code: Rust tests in each crate, Python tests in
`bindings/python/tests` and `adapters/freecad/tests`.
