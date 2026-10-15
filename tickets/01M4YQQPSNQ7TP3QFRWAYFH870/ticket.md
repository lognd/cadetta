+++
id = "01M4YQQPSNQ7TP3QFRWAYFH870"
title = "sync and names: invalid bindings file should exit 2, not report R005 (adapters.md 3)"
type = "bug"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T02:54:49Z"
updated = "2026-10-15T02:54:49Z"
idempotency_key = "sync-names-bindings-exit2"
scope = ["crates/cadetta-cli/src/cmd/sync.rs", "crates/cadetta-cli/src/cmd/names.rs", "crates/cadetta-cli/src/cmd/emit.rs", "crates/cadetta-cli/tests/**", "changelog.d/**", "docs/**"]

[[acceptance]]
text = "sync and names exit 2 naming the bindings file on an invalid bindings file or locator"
bound = false
+++

Follow-up to ~3F67WS1: emit now exits 2 on an invalid bindings file/locator; sync and names still report R005 with exit 0. Spec says any command stops with exit 2.
