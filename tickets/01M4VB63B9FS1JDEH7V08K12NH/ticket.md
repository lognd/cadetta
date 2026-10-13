+++
id = "01M4VB63B9FS1JDEH7V08K12NH"
title = "Pattern count members count_x, count_y and count"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 2
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T19:17:46Z"
updated = "2026-10-13T23:05:47Z"
persona = "design engineer"
capability = "bind a native pattern's instance count to the design's count"
outcome_text = "a count is written once"
idempotency_key = "d15:pattern-counts"
scope = ["crates/cadetta-geometry/src/pattern.rs", "crates/cadetta-sema/src/point.rs", "tests/conformance/geometry/**", "tests/conformance/known-gaps.toml"]

[[links]]
kind = "blocked-by"
target = "01M4VB636BKBDGRW3NBMGK7CBZ"

[[acceptance]]
text = "Given rect_pattern and circle_pattern, when evaluated, then count_x, count_y and count are Count members that bind and emit, and a count not known before evaluation is an error (geometry.md 3)"
bound = true
+++
