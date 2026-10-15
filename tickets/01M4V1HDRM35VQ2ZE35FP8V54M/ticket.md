+++
id = "01M4V1HDRM35VQ2ZE35FP8V54M"
title = "Quantities: cbrt unknown function (EVAL)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:29:11Z"
updated = "2026-10-15T02:00:32Z"
idempotency_key = "Z48-cbrt"
scope = ["crates/cadetta-sema/**", "tests/conformance/known-gaps.toml", "changelog.d/01M4V1HDRM35VQ2ZE35FP8V54M.*"]

[[acceptance]]
text = "cbrt is a known function: exact for perfect cubes, Q011 for a non-cube dimension"
bound = true
+++

found while working ~Z48GXKX. quantities.md 4.1 table lists cbrt. Fixtures q011-cbrt, kind-row-length-from-cbrt.
