+++
id = "01M4VPEFSVBXSEKWHTQ48SAEKA"
title = "names reads CAD so unbound and implicit link/frame/reference states are reachable"
type = "task"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-13T22:34:35Z"
updated = "2026-10-15T15:09:10Z"
scope = ["crates/cadetta-cli/src/cmd/names.rs"]

[[acceptance]]
text = "Given an adapter that can read, when cadetta names runs, then paths with no locator are unbound and native-link discoveries show implicit link, frame or reference; an adapter that cannot read keeps unknown (config-cli.md 3)"
bound = false
+++

found while working ~4T5YM3Y: names never launches CAD, so every path without a locator reports unknown and discovery-based implicit states are unreachable.
