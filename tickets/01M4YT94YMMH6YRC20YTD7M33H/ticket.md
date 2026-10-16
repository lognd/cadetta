+++
id = "01M4YT94YMMH6YRC20YTD7M33H"
title = "Wire csv read_bound (R016) into the CLI read path"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T03:39:18Z"
updated = "2026-10-16T09:17:51Z"
idempotency_key = "wire-csv-read-bound"
scope = ["crates/cadetta-cli/src/**", "crates/cadetta-adapter/src/builtin/csv.rs"]

[[acceptance]]
text = "The CLI csv read path calls read_bound and an unbound-name row exits with R016."
bound = false
+++

read_bound in builtin/csv.rs (from ~BB49Z35) has no caller; the CLI csv read path must use it so unbound rows raise R016.
