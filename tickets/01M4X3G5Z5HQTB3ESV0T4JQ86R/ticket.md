+++
id = "01M4X3G5Z5HQTB3ESV0T4JQ86R"
title = "clippy needless_borrows_for_generic_args in cadetta-cli pkg/resolve.rs"
type = "chore"
category = "in-progress"
priority = "low"
points = 1
reporter = "Claude"
created = "2026-10-14T11:41:57Z"
updated = "2026-10-14T11:50:53Z"
idempotency_key = "clippy-resolve-339"
scope = ["crates/cadetta-cli/src/pkg/resolve.rs"]

[[acceptance]]
text = "Given cargo clippy --workspace, when run, then pkg/resolve.rs reports no needless_borrows_for_generic_args"
bound = false
+++
