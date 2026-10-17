+++
id = "01M558W4WHGD721S1YG4V111FE"
title = "Transient RPC_E_SERVERFAULT on first drive read"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T15:49:47Z"
updated = "2026-10-17T16:54:14Z"
scope = ["adapters/solidworks/**"]

[[acceptance]]
text = "A read that hits a transient COM fault (RPC_E_SERVERFAULT) is retried with bounded backoff and succeeds; non-transient errors are not retried."
bound = true
+++

Found while working ~NEHY9X7 (nova live smoke). First read after attach failed once, then succeeded.
