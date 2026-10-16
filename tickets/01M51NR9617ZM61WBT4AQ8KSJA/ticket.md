+++
id = "01M51NR9617ZM61WBT4AQ8KSJA"
title = "emit: a pure reference does not inherit its referent's emit point (quantities.md 5.6)"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-16T06:17:54Z"
updated = "2026-10-16T12:09:56Z"
labels = ["creates:crates/cadetta-cli/src/cmd/emit.rs", "creates:crates/cadetta-sema/src/lower.rs"]
scope = ["crates/cadetta-cli/src/cmd/emit.rs", "crates/cadetta-sema/src/lower.rs", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "quantities/emit-point-inherited conformance fixture passes and is removed from known-gaps.toml"
bound = true
+++

found while working ~ZV526KN. Fixture quantities/emit-point-inherited: 'a = 12mm H7 #[emit(mid)]; b = a' emits b at nominal 12; quantities.md 5.6 says a pure reference inherits its referent's emit point (12.009). Likely sema lowering / emit point resolution (crates/cadetta-cli/src/cmd/emit.rs resolve_point input, lower.rs). Un-gap quantities/emit-point-inherited when fixed.
