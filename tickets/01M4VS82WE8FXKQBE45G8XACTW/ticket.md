+++
id = "01M4VS82WE8FXKQBE45G8XACTW"
title = "Treat tests/conformance/known-gaps.toml as an append-shared file for leases"
type = "chore"
category = "in-progress"
priority = "medium"
points = 1
parent = "01M4SPWFRR9DAEM60DX796GVPN"
reporter = "lognd"
created = "2026-10-13T23:23:31Z"
updated = "2026-10-13T23:23:35Z"
idempotency_key = "d15:known-gaps-shared"
scope = ["frob.toml"]

[[acceptance]]
text = "Given frob.toml [lease] shared_files listing the default lockfiles plus tests/conformance/known-gaps.toml, when two tickets both scope known-gaps.toml, then frob work no longer refuses for that overlap, and both the local frob and uvx --from frob==0.532.0 frob check accept the config"
bound = false
+++
