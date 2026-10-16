+++
id = "01M4YR871RH1TPH8854X50QNG4"
title = "Lower require items into IR so require sign-off obligations exist"
type = "bug"
category = "todo"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T03:03:50Z"
updated = "2026-10-16T08:24:08Z"
scope = ["crates/cadetta-sema/src/lower.rs", "crates/cadetta-ir/src/**", "docs/spec/ir.md", "crates/cadetta-cli/src/universe.rs", "changelog.d/**"]
+++

found while working ~Y0NVZAF: sema group_checks filters out require (!r.obligation), so ir.checks never carries them; ir.md says requires[] has no entity shape. CLI Universe::of_with already marks checks whose path is in analysis.suppressions.require_paths as require obligations (O003 via lint), but it is inert until IR carries requires. Needs a spec decision on the IR shape.
