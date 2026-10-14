+++
id = "01M4Y2Z99SXRHPTNQS11392B89"
title = "Emit lints G005 and G008 from placement float"
type = "task"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-14T20:51:57Z"
updated = "2026-10-14T20:53:30Z"
scope = ["crates/cadetta-lint/src/rules/g008.rs", "crates/cadetta-lint/src/engine.rs", "tests/conformance/lints/G008/**"]

[[links]]
kind = "blocked-by"
target = "01M4Y2Z95B99FTVX68T4XTSTXR"

[[acceptance]]
text = "Given a one-point pattern or s = 0 on a non-locating joint, when linted, then theta is 180deg, dependent checks are undecided and G008 fires (geometry.md 5.2; lints.md G008; takes over ~HWJP0GQ acceptance 3 emission, using float::unconstrained_rotation)"
bound = false
+++

Found while working ~HWJP0GQ. float::unconstrained_rotation is the G008 condition; this ticket emits the lint from the lowered EdgeFloat. G005 stays in ~XZD1Y50.
