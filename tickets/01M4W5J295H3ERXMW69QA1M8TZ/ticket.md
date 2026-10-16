+++
id = "01M4W5J295H3ERXMW69QA1M8TZ"
title = "Text members (callout) in the IR and runner; fmt over instance members"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-14T02:58:41Z"
updated = "2026-10-16T14:34:58Z"
scope = ["crates/cadetta-sema/**", "tests/conformance/stdlib/joint-callout.*", "tests/conformance/stdlib/joint-vendor-callout.*", "crates/cadetta-ir/src/**", "crates/cadetta-cli/tests/conformance/exec.rs", "docs/spec/ir.md", "docs/schemas/cadetta.ir.1.json", "bindings/python/python/cadetta/models/ir.py", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "Given joint-callout and joint-vendor-callout, when the conformance runner runs, then the Text callout is compared"
bound = true
+++

found while working ~646D1T7: Text values are not lowered to IR members; the cli runner (crates/cadetta-cli) must read them too.
