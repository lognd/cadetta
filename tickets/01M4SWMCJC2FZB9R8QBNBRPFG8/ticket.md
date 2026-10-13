+++
id = "01M4SWMCJC2FZB9R8QBNBRPFG8"
title = "Adapter ids, modules and paths scoping"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 3
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:44:11Z"
updated = "2026-10-13T09:38:45Z"
persona = "design engineer"
capability = "split one design across several files"
idempotency_key = "replan14:ad-scope"
labels = ["creates:crates/cadetta-adapter/src/scope.rs"]
scope = ["crates/cadetta-adapter/src/scope.rs", "crates/cadetta-adapter/src/lib.rs", "docs/spec/adapters.md"]

[[links]]
kind = "blocked-by"
target = "01M4SPWRGXS8XKJCF2H8FDKPKC"

[[links]]
kind = "blocked-by"
target = "01M4SPWSTRG1FFAJ0XM89ETBBN"

[[acceptance]]
text = "Given two [[adapter]] blocks with the same name, when id is missing, then it is a configuration error; id defaults to name (config-cli.md 2)"
bound = true

[[acceptance]]
text = "Given modules and paths globs on an adapter, when emitting or syncing, then only matching canonical paths reach the adapter (adapters.md 3.2)"
bound = true
+++
