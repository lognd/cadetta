+++
id = "01M51NR9617ZM61WBT4AQ8KSJA"
title = "emit: #[role(..)] attribute and reference inheritance do not reach the emit point policy (mmc/lmc E010)"
type = "bug"
category = "todo"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-16T06:17:54Z"
updated = "2026-10-16T06:17:54Z"
labels = ["creates:crates/cadetta-cli/src/cmd/emit.rs", "creates:crates/cadetta-sema/src/lower.rs"]
scope = ["crates/cadetta-cli/src/cmd/emit.rs", "crates/cadetta-sema/src/lower.rs"]

[[acceptance]]
text = "quantities/emit-point-lmc-hole, emit-point-lmc-shaft and emit-point-inherited conformance fixtures pass and are removed from known-gaps.toml"
bound = false
+++

found while working ~ZV526KN. Fixtures quantities/emit-point-lmc-hole, emit-point-lmc-shaft: '12mm [+0.029mm +0.011mm] #[role(hole), emit(lmc)]' still fails emit with E010 (member.role is not set from the role attribute when emit reads it, crates/cadetta-cli/src/cmd/emit.rs role_of). quantities/emit-point-inherited: 'b = a' where a is #[emit(mid)] emits b at nominal 12, spec quantities.md 5.6 says a pure reference inherits the referent's emit point. Likely sema lowering (lower.rs role_from_attrs / reference provenance); related ~B08JM3F (class-letter role). Un-gap the three fixtures when fixed.
