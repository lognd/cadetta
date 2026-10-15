+++
id = "01M4XDM9WRFB7RFEF2X6RBPR2Y"
title = "sync: unit-converting normalization"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-14T14:38:57Z"
updated = "2026-10-15T05:06:53Z"
scope = ["crates/cadetta-cli/src/cmd/sync.rs"]

[[acceptance]]
text = "Same-dimension units convert exactly to the native unit before comparing"
bound = true

[[acceptance]]
text = "Inexact or incompatible text is left unchanged"
bound = true
+++

found while working ~D4T6ATZ: units compared textually
