+++
id = "01M4YPDTD1977TMY4M2PPPNSDD"
title = "Thread a configurable adapter timeout and apply it in sync/import adapter callers"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:31:57Z"
updated = "2026-10-15T02:31:57Z"
scope = ["crates/cadetta-cli/src/cmd/import.rs", "crates/cadetta-cli/src/cmd/check_drift.rs", "crates/cadetta-adapter/src/rpc/client.rs", "docs/spec/config-cli.md", "docs/spec/adapters.md"]

[[acceptance]]
text = "sync and import bound adapter reads by a configurable timeout"
bound = false

[[acceptance]]
text = "Timeout setting documented in the spec"
bound = false
+++

Found while working ~78B88BT. check now kills a silent adapter after a fixed 60s (ADAPTER_TIMEOUT in check_drift.rs, ProcessClient::spawn_with_timeout). sync and import still call ProcessClient::spawn with no timeout, so a silent adapter can hang them. Add a timeout setting (config and/or --timeout flag), use spawn_with_timeout in those callers, map the timeout to the adapter-unresponsive error, and document it. Tests: a silent adapter makes sync and import fail within the configured bound.
