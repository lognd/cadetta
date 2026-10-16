+++
id = "01M4Z3FGTSPMCJJJ3093T0ABJY"
title = "language: waiver on an item path must cover diagnostics beneath it (i001-fixed must-not-fire: C001 waived on acme.demo::A, W004 + C001 still reported at A.load_rating)"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T06:20:04Z"
updated = "2026-10-16T13:05:47Z"
scope = ["crates/cadetta-lint/**", "crates/cadetta-lint/src/engine.rs", "docs/spec/lints.md", "tests/conformance/language/waiver-item-path*"]
+++

Found in ~8NXQSDY. 'waive C001 on acme.demo::A' does not match C001 at acme.demo::A.load_rating, so W004 fires and C001 stays an error.
