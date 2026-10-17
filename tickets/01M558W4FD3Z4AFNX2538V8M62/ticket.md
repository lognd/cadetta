+++
id = "01M558W4FD3Z4AFNX2538V8M62"
title = "cadetta names opens the project file in the running SOLIDWORKS"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T15:49:47Z"
updated = "2026-10-17T16:51:31Z"
scope = ["crates/**", "docs/spec/config-cli.md"]

[[acceptance]]
text = "Given a reading adapter, when cadetta names runs without --observe, then the adapter is not spawned and states stay unknown; with --observe it reads CAD."
bound = true
+++

Found while working ~NEHY9X7 (nova live smoke). Running names against a project opened the file in the user's SOLIDWORKS session.
