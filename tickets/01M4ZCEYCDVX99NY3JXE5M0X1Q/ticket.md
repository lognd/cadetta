+++
id = "01M4ZCEYCDVX99NY3JXE5M0X1Q"
title = "emit rejects a tolerance locator without a dimension kind (config error, exit 2)"
type = "task"
category = "done"
outcome = "done"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T08:57:02Z"
updated = "2026-10-16T15:37:36Z"
scope = ["crates/cadetta-adapter/src/locator.rs", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "A tolerance on a non-dimension locator is rejected as a config error naming the binding path (unit test)"
bound = true
+++

adapters/tol-locator-needs-dimension expects exit 2 naming the path or key; runner key names_path_or_key is also unobserved. Implementation does not validate yet. Split from ~SK1B9S3.
