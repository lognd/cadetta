+++
id = "01M4Z92C25BYMP5JYSX9WDH9XE"
title = "Part-array instance members are not lowered to IR paths, so emitters and bindings see none"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T07:57:44Z"
updated = "2026-10-15T08:20:47Z"
labels = ["creates:crates/cadetta-sema/src/lower_array.rs"]
scope = ["crates/cadetta-sema/src/**", "docs/spec/ir.md", "tests/conformance/known-gaps.toml", "crates/cadetta-sema/src/lower_array.rs", "tests/conformance/emitters/json/name-mangled-path.expect.json"]

[[acceptance]]
text = "Part-array instance members lower to Parent/part[i].member IR members"
bound = false
+++

found while working ~6ES67RW: with valid syntax 'part axle[2]: Axle' emit csv/json/sw-equations write no Rover/axle[i].member rows and [*] / [1] bindings never match. Blocks fixtures adapters/star-binding-emitted-once-at-instance-0, adapters/star-mangle-drops-segment, adapters/r012-star-binding-differing-instances (must-not-fire), emitters/{csv,json}/name-mangled-path, emitters/sw-equations/naming-component-field.
