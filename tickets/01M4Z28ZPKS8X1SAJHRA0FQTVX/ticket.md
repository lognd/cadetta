+++
id = "01M4Z28ZPKS8X1SAJHRA0FQTVX"
title = "language: F002 sub-nanometre, fit clearance member, process-drawn value"
type = "task"
category = "todo"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T05:59:01Z"
updated = "2026-10-15T05:59:01Z"
idempotency_key = "split-0N41BNX-fit"
scope = ["crates/cadetta-fits/**"]

[[acceptance]]
text = "The three fixtures pass conformance without known-gaps entries"
bound = false
+++

Split from ~0N41BNX. Fixtures: language/f002-sub-nanometre, fit-clearance-member, process-drawn-value. Remove the fixtures' entries from tests/conformance/known-gaps.toml (own hunks only).
