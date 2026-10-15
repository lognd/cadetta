+++
id = "01M4YMKH7NQW964KS1C2GJX813"
title = "Quantities: user-declared kinds are not registered in the KindTable (kind-row-length-from-cbrt Q003)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:00:07Z"
updated = "2026-10-15T02:02:33Z"
labels = ["creates:crates/cadetta-cli/tests/user_kinds.rs"]
scope = ["crates/cadetta-sema/**", "crates/cadetta-cli/src/pipeline.rs", "docs/spec/quantities.md", "tests/conformance/known-gaps.toml", "changelog.d/01M4YMKH7NQW964KS1C2GJX813.fixed.md", "crates/cadetta-cli/tests/user_kinds.rs"]

[[acceptance]]
text = "A user-declared kind is registered so assigning a differently-kinded value reports Q003"
bound = true

[[acceptance]]
text = "The kind-row-length-from-cbrt known gap is removed"
bound = false
+++

found while working ~FP8V54M. pipeline uses KindTable::builtin(); user 'kind Sink = Length' decls are never registered so kindcheck target() returns None and Q003 is not raised (fixture quantities/kind-row-length-from-cbrt).
