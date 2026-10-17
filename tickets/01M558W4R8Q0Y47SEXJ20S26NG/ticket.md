+++
id = "01M558W4R8Q0Y47SEXJ20S26NG"
title = "read_current returns None for Skipped statuses"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T15:49:47Z"
updated = "2026-10-17T16:25:49Z"
scope = ["crates/**"]

[[acceptance]]
text = "Given a locator whose push status is skipped, when the document is read back, then its value is returned and recorded as synced"
bound = false
+++

Found while working ~NEHY9X7 (nova live smoke). Re-sync can then report a spurious mismatch.
