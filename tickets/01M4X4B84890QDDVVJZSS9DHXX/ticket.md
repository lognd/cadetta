+++
id = "01M4X4B84890QDDVVJZSS9DHXX"
title = "CLI: impact --since REV and --worktree"
type = "task"
category = "todo"
priority = "medium"
reporter = "Claude"
created = "2026-10-14T11:56:44Z"
updated = "2026-10-14T11:59:55Z"
+++

Found while working ~EWBDP66: bindings.md 3.2 lists project.impact_since(rev, worktree) over impact --since/--worktree, but the CLI impact verb only takes PATH or --swap. The Python method is deferred until the CLI flags exist.
