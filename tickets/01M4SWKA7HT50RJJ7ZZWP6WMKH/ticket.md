+++
id = "01M4SWKA7HT50RJJ7ZZWP6WMKH"
title = "check --watch with newline-delimited cadetta.diag/1"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:43:36Z"
updated = "2026-10-13T14:05:40Z"
persona = "design engineer"
capability = "get editor problems without an LSP"
idempotency_key = "replan14:lint-watch"
labels = ["creates:crates/cadetta-cli/src/cmd/check_watch.rs"]
scope = ["crates/cadetta-cli/src/cmd/check_watch.rs", "crates/cadetta-cli/src/cmd/check.rs", "crates/cadetta-cli/src/cmd/mod.rs", "docs/spec/config-cli.md"]

[[links]]
kind = "blocked-by"
target = "01M4SPWT3FP4SN4RTCHE631Y52"

[[links]]
kind = "blocked-by"
target = "01M4SWK9V5CYNNVTAS3PE4EC23"

[[acceptance]]
text = "Given check --format json --watch, when a source file changes, then a re-run emits one cadetta.diag/1 per line sorted by (file, line, code) (lints.md 1.2; config-cli.md 5)"
bound = true

[[acceptance]]
text = "Given an unchanged run, when watching, then nothing new is emitted twice for identical records (config-cli.md 5)"
bound = true
+++
