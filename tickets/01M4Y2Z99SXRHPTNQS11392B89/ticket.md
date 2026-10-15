+++
id = "01M4Y2Z99SXRHPTNQS11392B89"
title = "Emit lint G008 from the placement float"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-14T20:51:57Z"
updated = "2026-10-15T10:15:42Z"
labels = ["creates:crates/cadetta-lint/src/rules/g008.rs", "creates:tests/conformance/lints/G008/**"]
scope = ["crates/cadetta-lint/src/engine.rs", "crates/cadetta-lint/src/rules/g008.rs", "tests/conformance/lints/G008/**", "crates/cadetta-sema/src/geom.rs", "crates/cadetta-sema/src/eval.rs", "crates/cadetta-sema/tests/bolt_float.rs", "docs/spec/geometry.md", "tests/conformance/known-gaps.toml", "tests/conformance/geometry/g008-*"]

[[links]]
kind = "blocked-by"
target = "01M4Y2Z95B99FTVX68T4XTSTXR"

[[acceptance]]
text = "Given a one-point pattern or s = 0 on a non-locating joint, when linted, then theta is 180deg, dependent checks are undecided and G008 fires (geometry.md 5.2; lints.md G008; takes over ~HWJP0GQ acceptance 3 emission, using float::unconstrained_rotation)"
bound = false
+++

Found while working ~HWJP0GQ. float::unconstrained_rotation is the G008 condition; this ticket emits the lint from the lowered EdgeFloat. G005 stays in ~XZD1Y50.
