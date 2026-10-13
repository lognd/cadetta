+++
id = "01M4VS6WG57MAD0T8GK956NGG9"
title = "Wire cadetta_geometry::aligned/distance into check evaluation and emit G004"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T23:22:52Z"
updated = "2026-10-13T23:22:52Z"
scope = ["crates/cadetta-sema/src/**", "crates/cadetta-lint/src/**"]
+++

found while working ~5DYGF7F: aligned/distance are implemented in cadetta-geometry but no crate calls them; sema does not evaluate aligned or place, so G004 and the section 1 fixtures stay known gaps (~MCJWA2D, ~H0EKNH2). Once wired, remove geometry/aligned-tolerance-range, geometry/distance-axial-component and the g004 entries from known-gaps.toml.
