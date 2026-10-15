+++
id = "01M4V1X51H5H3BXZ9FTW41EHE6"
title = "check omits R008 info for unavailable external adapter"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:35:36Z"
updated = "2026-10-15T02:49:08Z"
scope = ["crates/cadetta-cli/**"]

[[acceptance]]
text = "check reports R008 info (exit 0) when an external adapter command is missing"
bound = false

[[acceptance]]
text = "a bare adapter command name is not PATH-searched and yields R008 info"
bound = false
+++

adapters.md 2.3: check must report R008 (info) when an external adapter command is missing; binary emits nothing. Gap entries adapters/r008-check-adapter-unavailable and adapters/r008-no-path-search. found while working ~YNT9TWR
