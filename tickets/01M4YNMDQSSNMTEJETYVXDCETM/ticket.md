+++
id = "01M4YNMDQSSNMTEJETYVXDCETM"
title = "Fix conformance/ir fixtures: placements use 'on' not at/path; observed must-not-fire fixtures lack bindings"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:18:04Z"
updated = "2026-10-15T02:21:05Z"
scope = ["tests/conformance/ir/**", "crates/cadetta-cli/tests/ir_entries.rs", "changelog.d/*"]

[[acceptance]]
text = "All ir corpus must-not-fire fixtures validate (exit 0) with no skip list; observed-binding rule still enforced"
bound = false
+++

Found while doing 5ZFM9K6. Schema-invalid: datum-variants-chain, op-flip-axis, op-one-key, op-rotate-axis, path-star-bindings-only (must-not-fire). Observed must-not-fire fixtures with no bindings entry (ir.md 6 requires one per observed member): observed-collision-suffix, -component, -deps-empty, -no-citations, -no-parts, -path-safe, -provenance. Tests exclude them in crates/cadetta-cli/tests/ir_entries.rs SCHEMA_GAPS.
