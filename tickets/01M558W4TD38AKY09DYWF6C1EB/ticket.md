+++
id = "01M558W4TD38AKY09DYWF6C1EB"
title = "Failed atomic restore can leave a half-pushed document"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T15:49:47Z"
updated = "2026-10-17T16:42:15Z"
scope = ["adapters/solidworks/**", "docs/spec/solidworks.md"]

[[acceptance]]
text = "Given a rebuild failure, when an undo step is refused or throws, then the remaining undos still run and R004 reports the restore also failed"
bound = true
+++

Found while working ~NEHY9X7 (nova live smoke). Seen only by reading the restore path; not reproduced live.
