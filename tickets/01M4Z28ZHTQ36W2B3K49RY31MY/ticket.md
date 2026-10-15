+++
id = "01M4Z28ZHTQ36W2B3K49RY31MY"
title = "language: U002 unused-private-item fixtures (u002-*)"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T05:59:01Z"
updated = "2026-10-15T06:22:30Z"
idempotency_key = "split-0N41BNX-u002"
scope = ["crates/cadetta-lint/**", "crates/cadetta-cli/src/lint_facts.rs", "tests/conformance/**", "docs/spec/lints.md"]

[[acceptance]]
text = "The three u002 fixtures pass conformance without known-gaps entries"
bound = true
+++

Split from ~0N41BNX. Fixtures: language/u002-unused-private, packages/u002-not-i004-for-private-component, packages/u002-used-in-descendant. Remove the fixtures' entries from tests/conformance/known-gaps.toml (own hunks only).
