+++
id = "01M4V1HDNKJ9095119JPVAKYG1"
title = "Quantities: unknown kind name accepted (a: Foo = 5mm should be E002)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:29:11Z"
updated = "2026-10-15T02:00:18Z"
idempotency_key = "Z48-kindname"
scope = ["crates/cadetta-sema/**", "tests/conformance/known-gaps.toml", "changelog.d/01M4V1HDNKJ9095119JPVAKYG1.*"]

[[acceptance]]
text = "A declared type that is neither a kind nor a known type is reported E002"
bound = true
+++

found while working ~Z48GXKX. quantities.md 4.4. Fixture e002-kind.
