+++
id = "01M4Z3FGTSPMCJJJ3093T0ABJY"
title = "language: waiver on an item path must cover diagnostics beneath it (i001-fixed must-not-fire: C001 waived on acme.demo::A, W004 + C001 still reported at A.load_rating)"
type = "task"
category = "done"
outcome = "done"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T06:20:04Z"
updated = "2026-10-16T13:22:12Z"
scope = ["crates/cadetta-lint/**", "crates/cadetta-lint/src/engine.rs", "docs/spec/lints.md", "tests/conformance/language/waiver-item-path*"]

[[acceptance]]
text = "A waiver on an item path covers diagnostics at paths beneath it (lints.md 2.1)"
bound = true
+++

Found in ~8NXQSDY. 'waive C001 on acme.demo::A' does not match C001 at acme.demo::A.load_rating, so W004 fires and C001 stays an error.
