+++
id = "01M4ZQ92JVS7DMCGDSCWZAC967"
title = "emit: run external adapters in cadetta emit"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T12:06:04Z"
updated = "2026-10-15T13:00:29Z"
scope = ["crates/cadetta-cli/src/cmd/emit.rs", "crates/cadetta-cli/src/cmd/sync.rs"]

[[acceptance]]
text = "cadetta emit spawns external adapters and sends emit, reporting their files"
bound = false
+++

Found while working ~5HP91AE: emit.rs (~line 1077) still skips external adapters; sync now runs them via emit_external.
