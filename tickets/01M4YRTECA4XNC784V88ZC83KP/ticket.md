+++
id = "01M4YRTECA4XNC784V88ZC83KP"
title = "check: resolve git dependencies (currently skipped with a warning)"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T03:13:47Z"
updated = "2026-10-15T03:13:47Z"
scope = ["crates/cadetta-cli/**"]

[[acceptance]]
text = "cadetta check loads git dependencies from the lock-pinned checkout"
bound = false
+++

found while working ~992PZ41: Project::load only loads path dependencies; git sources need an injected fetch (see cmd/update.rs git_checkout); use of a git dependency stays E002 in check.
