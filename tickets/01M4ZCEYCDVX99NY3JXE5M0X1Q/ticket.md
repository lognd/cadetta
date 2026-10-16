+++
id = "01M4ZCEYCDVX99NY3JXE5M0X1Q"
title = "emit rejects a tolerance locator without a dimension kind (config error, exit 2)"
type = "task"
category = "todo"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T08:57:02Z"
updated = "2026-10-16T15:08:04Z"
scope = ["crates/**"]
+++

adapters/tol-locator-needs-dimension expects exit 2 naming the path or key; runner key names_path_or_key is also unobserved. Implementation does not validate yet. Split from ~SK1B9S3.
