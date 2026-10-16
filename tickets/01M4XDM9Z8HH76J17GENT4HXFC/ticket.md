+++
id = "01M4XDM9Z8HH76J17GENT4HXFC"
title = "sync: ref-mode locator support"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-14T14:38:58Z"
updated = "2026-10-16T07:43:11Z"
scope = ["crates/cadetta-cli/src/cmd/sync.rs", "docs/spec/adapters.md"]

[[acceptance]]
text = "A ref-mode dimension locator is link-checked by sync: R003 when absent or no longer linked to its target global, never pushed"
bound = true
+++

found while working ~D4T6ATZ: ref-mode locators skipped
