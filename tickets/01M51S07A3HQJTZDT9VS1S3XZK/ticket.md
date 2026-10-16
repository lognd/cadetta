+++
id = "01M51S07A3HQJTZDT9VS1S3XZK"
title = "Fix stale entries in conformance known-gaps.toml"
type = "bug"
category = "todo"
priority = "high"
points = 2
reporter = "lognd"
created = "2026-10-16T07:14:40Z"
updated = "2026-10-16T07:14:40Z"
scope = ["tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "conformance_corpus_matches_the_spec passes"
bound = false
+++

conformance_corpus_matches_the_spec fails on main: stale gaps (e.g. ~SK4E68X, ~E5M0X1Q).
