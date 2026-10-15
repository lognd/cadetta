+++
id = "01M4V3F2A8PNNRXND41CXE9G4M"
title = "S007 and Q001 not raised for solver in-expression forms"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T17:02:51Z"
updated = "2026-10-15T02:28:04Z"
scope = ["crates/cadetta-sema/**", "crates/cadetta-syntax/**", "crates/cadetta-solve/**", "crates/cadetta-cli/src/pipeline.rs", "tests/conformance/solvers/**", "tests/conformance/lints/**", "tests/conformance/known-gaps.toml", "docs/**"]

[[acceptance]]
text = "solvers.md 1: in forms string/boolean/array/comparison/logic/range/if/match raise S007 and bare numbers or bare additive operands raise Q001 in cadetta check (fixtures solvers/s007-in-*, q001-in-*; found while working ~KBYTYF0)"
bound = false
+++
