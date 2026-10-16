+++
id = "01M4YR871RH1TPH8854X50QNG4"
title = "Lower require items into IR so require sign-off obligations exist"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T03:03:50Z"
updated = "2026-10-16T08:43:50Z"
scope = ["crates/cadetta-sema/src/lower.rs", "crates/cadetta-ir/src/**", "docs/spec/ir.md", "crates/cadetta-cli/src/universe.rs", "changelog.d/**"]

[[acceptance]]
text = "A require lowers to requires[] with message, citation and verdicts, not into checks[]"
bound = true

[[acceptance]]
text = "The CLI universe lists IR requires as require obligations"
bound = false
+++

found while working ~Y0NVZAF: sema group_checks filters out require (!r.obligation), so ir.checks never carries them; ir.md says requires[] has no entity shape. CLI Universe::of_with already marks checks whose path is in analysis.suppressions.require_paths as require obligations (O003 via lint), but it is inert until IR carries requires. Needs a spec decision on the IR shape.
