+++
id = "01M4YB06FSER6VHW3RBC0B2X17"
title = "Observe facts for R007, R011, R012 and run them under check"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-14T23:12:16Z"
updated = "2026-10-16T08:11:24Z"
scope = ["crates/cadetta-adapter/**", "crates/cadetta-cli/**", "crates/cadetta-lint/**", "docs/**", "changelog.d/**"]

[[links]]
kind = "blocked-by"
target = "01M4TA3C87N60Y711163RK3GPH"

[[acceptance]]
text = "read carries sketch-plane/datum facts and cadetta check reports R007 for a coordinate sketch off its datum plane"
bound = false

[[acceptance]]
text = "read carries assembly slot component files and cadetta check reports R011 on a swapped component"
bound = false

[[acceptance]]
text = "cadetta check reports R012 for a [*] binding over instances with differing emitted values"
bound = false
+++

Follow-up to ~3RK3GPH: check_r007/check_r011/check_r012/check_r013 exist in cadetta-lint r_adapter.rs but the observed-IR read result has no sketch plane, assembly slot or per-instance facts to build their inputs, so check cannot run them. Extend the read response schema (adapters.md) and cadetta-cli check_drift::lint_probe.
