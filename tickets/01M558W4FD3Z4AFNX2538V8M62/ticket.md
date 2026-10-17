+++
id = "01M558W4FD3Z4AFNX2538V8M62"
title = "cadetta names opens the project file in the running SOLIDWORKS"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T15:49:47Z"
updated = "2026-10-17T16:50:12Z"
scope = ["crates/**"]

[[acceptance]]
text = "Given a reading adapter, when cadetta names runs without --observe, then the adapter is not spawned and states stay unknown; with --observe it reads CAD."
bound = false
+++

Found while working ~NEHY9X7 (nova live smoke). Running names against a project opened the file in the user's SOLIDWORKS session.
