+++
id = "01M4XDMA1D78TRJB1CB5HP91AE"
title = "sync: run external non-drive adapter emit"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-14T14:38:58Z"
updated = "2026-10-15T12:04:25Z"
scope = ["crates/cadetta-cli/src/cmd/sync.rs"]

[[acceptance]]
text = "An external adapter declaring emit but not drive has emit run by cadetta sync and its files reported"
bound = true
+++

found while working ~D4T6ATZ: only built-in sw-equations emitted
