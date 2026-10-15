+++
id = "01M4ZSTX34B5RTHS2Y5TND4WKS"
title = "O004 open sync obligation for a json/csv/sw-equations binding without drive"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T12:50:45Z"
updated = "2026-10-15T16:08:54Z"
scope = ["crates/cadetta-cli/src/universe.rs", "crates/cadetta-lint/src/rules/o004_w005.rs", "crates/cadetta-obligations/**"]

[[acceptance]]
text = "cadetta check reports O004 (info by default) with reason 'unverifiable without drive' for a json/csv binding whose adapter lacks drive after sync"
bound = false
+++

Split from ~EPB7ZY9 (its scope is cadetta-adapter only). check_o004 only handles Stale sync obligations and universe::sync_states only emits Stale; Open-with-reason needs both extended. Spec: adapters.md 2, json.md 4, csv.md 4, obligations.md 2.3. universe.rs overlaps in-flight emit/sync/names/universe work.
