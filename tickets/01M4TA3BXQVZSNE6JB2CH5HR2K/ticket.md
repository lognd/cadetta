+++
id = "01M4TA3BXQVZSNE6JB2CH5HR2K"
title = "KindChecker uses EmptyEnv for cross-component names"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T09:39:33Z"
updated = "2026-10-14T21:56:02Z"
scope = ["crates/cadetta-sema", "crates/cadetta-cli/src", "docs/spec/quantities.md", "changelog.d"]

[[acceptance]]
text = "cadetta check reports Q002 for a parameter assigned a cross-component read of the wrong dimension (Length = Plate.mass)"
bound = true

[[acceptance]]
text = "cadetta check reports no Q or E diagnostic for a cross-component read of the matching dimension (Length = Plate.width)"
bound = true
+++

found while working ~E631Y52. pipeline wires no TypeEnv.
