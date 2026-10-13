+++
id = "01M4SPWRS0CDJT4GYC92AB1HCD"
title = "JSON-RPC 2.0 stdio protocol host"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T04:03:54Z"
updated = "2026-10-13T13:14:14Z"
persona = "cadetta contributor"
capability = "drive external adapters over stdio with initialize, emit, read, push, regen, shutdown"
outcome_text = "adapters can be written in any language"
labels = ["creates:crates/cadetta-adapter/src/rpc/**", "creates:docs/schemas/adapter-protocol.json"]
scope = ["crates/cadetta-adapter/src/rpc/**", "docs/schemas/adapter-protocol.json", "crates/cadetta-adapter/src/lib.rs", "crates/cadetta-adapter/Cargo.toml", "Cargo.lock", "docs/spec/adapters.md"]

[[links]]
kind = "blocks"
target = "01M4SPWSCZ06W3EW6B1RDPEZRQ"

[[acceptance]]
text = "Given an external adapter, when initialize is sent, then name, version, capabilities, native units, precision and platforms are returned"
bound = true

[[acceptance]]
text = "Given a mock adapter, when the full method sequence runs, then each result validates against the protocol schema"
bound = true

[[acceptance]]
text = "Given a version in cadetta.toml not matching initialize, when connecting, then it refuses (OQ-7)"
bound = true

[[acceptance]]
text = "Given the host, when it speaks JSON-RPC, then open, save and close are available for drive adapters and diagnostics reuse cadetta.diag/1 (adapters.md 5)"
bound = true
+++
