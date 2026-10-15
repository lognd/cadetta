+++
id = "01M4YR011TKARTCKMXB4P8A6ST"
title = "parser: one bad waive date token emits two E001 (cascade)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:59:22Z"
updated = "2026-10-15T03:21:36Z"
scope = ["crates/cadetta-syntax/**", "crates/cadetta-syntax/**"]

[[acceptance]]
text = "One malformed waive date token yields exactly one E001"
bound = false
+++

Split from ~8WN9V3R. A single malformed waive date token yields two E001 diagnostics; expected one.
