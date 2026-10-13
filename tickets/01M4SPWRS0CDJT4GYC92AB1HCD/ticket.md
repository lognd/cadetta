+++
id = "01M4SPWRS0CDJT4GYC92AB1HCD"
title = "JSON-RPC 2.0 stdio protocol host"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T04:03:54Z"
updated = "2026-10-13T04:03:54Z"
persona = "cadetta contributor"
capability = "drive external adapters over stdio with initialize, emit, read, push, regen, shutdown"
outcome_text = "adapters can be written in any language"
labels = ["creates:crates/cadetta-adapter/src/rpc/**", "creates:docs/schemas/adapter-protocol.json"]
scope = ["crates/cadetta-adapter/src/rpc/**", "docs/schemas/adapter-protocol.json"]

[[acceptance]]
text = "Given an external adapter, when initialize is sent, then name, version, capabilities, native units, precision and platforms are returned"
bound = false

[[acceptance]]
text = "Given a mock adapter, when the full method sequence runs, then each result validates against the protocol schema"
bound = false

[[acceptance]]
text = "Given a version in cadetta.toml not matching initialize, when connecting, then it refuses (OQ-7)"
bound = false
+++
