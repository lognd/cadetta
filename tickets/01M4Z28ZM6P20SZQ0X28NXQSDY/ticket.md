+++
id = "01M4Z28ZM6P20SZQ0X28NXQSDY"
title = "language: I001-I003 interface and variant semantics"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T05:59:01Z"
updated = "2026-10-15T06:08:38Z"
idempotency_key = "split-0N41BNX-i001"
scope = ["crates/cadetta-variants/**"]

[[acceptance]]
text = "The seven i001-i003 fixtures pass conformance without known-gaps entries"
bound = false
+++

Split from ~0N41BNX. Fixtures: language/i001-fixed, i001-missing-member, i001-optional-member, i001-wrong-kind, i002-alternate-not-viable, i002-never-cross-product, i003-alternate-undecided. Remove the fixtures' entries from tests/conformance/known-gaps.toml (own hunks only).
