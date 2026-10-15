+++
id = "01M4XDM9ST75VR0KRM3TDVNMAR"
title = "sync: design tolerance comparison"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-14T14:38:57Z"
updated = "2026-10-15T05:06:01Z"
scope = ["crates/cadetta-cli/src/cmd/sync.rs", "crates/cadetta-cli/src/cmd/emit.rs", "docs/spec/adapters.md", "changelog.d/*"]

[[acceptance]]
text = "A design tolerance is pushed with the value (symmetric or bilateral)"
bound = true

[[acceptance]]
text = "A tolerance edited in CAD is reported as R001"
bound = false

[[acceptance]]
text = "Tolerances are ignored without the tolerances feature"
bound = false
+++

found while working ~D4T6ATZ: ours_tol/theirs_tol are None
