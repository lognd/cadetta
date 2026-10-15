+++
id = "01M4V2K8RG8K1P774YVYBFVYDC"
title = "S007 not raised for untyped solver out port"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:47:40Z"
updated = "2026-10-15T02:28:08Z"
scope = ["crates/cadetta-sema/**", "crates/cadetta-syntax/**", "crates/cadetta-solve/**", "crates/cadetta-cli/src/pipeline.rs", "tests/conformance/solvers/**", "tests/conformance/lints/**", "tests/conformance/known-gaps.toml", "docs/**", "changelog.d/**"]

[[acceptance]]
text = "solvers.md 1: an out port without a kind raises S007, not a syntax error (fixtures lints/s007-solver-untyped-port, solvers/s007-out-without-kind, s007-out-kind-expression)"
bound = true
+++

Fixture lints/s007-solver-untyped-port: out without kind gives E001 instead of S007; in with non-quantity expression gives nothing. found while working ~T71T26K
