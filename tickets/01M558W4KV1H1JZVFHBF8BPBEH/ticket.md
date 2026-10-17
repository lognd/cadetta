+++
id = "01M558W4KV1H1JZVFHBF8BPBEH"
title = "Adapter reports observed IR file names bare"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T15:49:47Z"
updated = "2026-10-17T16:35:38Z"
scope = ["crates/**"]

[[acceptance]]
text = "Given an adapter reports an observed file by base name, when sync pushes, then the package-relative path is opened"
bound = true
+++

Found while working ~NEHY9X7 (nova live smoke). Worked around by FindUnderCwd in the drive server.
