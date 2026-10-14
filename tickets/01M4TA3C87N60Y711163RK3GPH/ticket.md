+++
id = "01M4TA3C87N60Y711163RK3GPH"
title = "Wire adapter read/drift lints (R001-R005, R008-R010) into check"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T09:39:34Z"
updated = "2026-10-14T23:12:41Z"
scope = ["crates/cadetta-adapter", "crates/cadetta-cli/**", "crates/cadetta-lint/**", "docs/**", "changelog.d/**"]

[[acceptance]]
text = "cadetta check spawns each readable adapter and reports R001 when the CAD value and the design both moved from the lock base"
bound = true

[[acceptance]]
text = "cadetta check is clean of R lints when the CAD value matches the design"
bound = true

[[acceptance]]
text = "cadetta check reports R003 for a bound locator missing from the native file, and R002 for an unbound native value"
bound = true

[[acceptance]]
text = "cadetta check reports R005 (warning) for a binding whose path matches no IR path"
bound = true

[[acceptance]]
text = "cadetta check reports an unstartable adapter as R008 at info severity and exits 0"
bound = true

[[acceptance]]
text = "cadetta check passes adapter-reported R009 diagnostics through"
bound = true
+++

found while working ~E631Y52. check drift is an injectable hook, default no-op.
