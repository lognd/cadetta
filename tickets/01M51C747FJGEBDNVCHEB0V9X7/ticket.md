+++
id = "01M51C747FJGEBDNVCHEB0V9X7"
title = "Conformance fixtures for language: process-rule kind, defaults precedence, case canon digits, fn/recursion, bound satisfies, keyword after dot, ISO class E001, waive date E001"
type = "task"
category = "done"
outcome = "done"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-16T03:31:15Z"
updated = "2026-10-16T06:22:21Z"
scope = ["tests/conformance/language/**", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "Fixture pairs exist for process-rule kind, defaults precedence, case canon digits, fn recursion, bound satisfies, keyword after dot, ISO class E001, waive date E001"
bound = true

[[acceptance]]
text = "Each fixture matches the spec or is gapped in known-gaps.toml with reason and ticket"
bound = true
+++

Remainder of ~4GVR0SD: W003/W004/S002/S004/O003 already covered by lints/ and obligations/ fixtures; EVAL part-array pairs landed in ~4GVR0SD.
