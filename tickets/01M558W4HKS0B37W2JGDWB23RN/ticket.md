+++
id = "01M558W4HKS0B37W2JGDWB23RN"
title = "import: OpenDoc6 fails on relative paths"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T15:49:47Z"
updated = "2026-10-17T16:39:48Z"
scope = ["crates/**"]

[[acceptance]]
text = "Given a relative native file path, when the CLI reads an adapter (import, names, sync), then the path sent over RPC is absolute."
bound = false
+++

Found while working ~NEHY9X7 (nova live smoke). Relative document paths fail to open; absolute paths work.
