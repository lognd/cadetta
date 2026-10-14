+++
id = "01M4TA3C87N60Y711163RK3GPH"
title = "Adapter read/drift lints (R rules) not implemented"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T09:39:34Z"
updated = "2026-10-14T23:12:08Z"
scope = ["crates/cadetta-adapter", "crates/cadetta-cli/**", "crates/cadetta-lint/**", "docs/**", "changelog.d/**"]

[[acceptance]]
text = "cadetta check spawns each readable adapter and reports R001 when the CAD value and the design both moved from the lock base"
bound = false
+++

found while working ~E631Y52. check drift is an injectable hook, default no-op.
