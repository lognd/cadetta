+++
id = "01M4YR9ZHDP7SF9KGMDBV5FGCF"
title = "solvers: freshness stdout and S005 fixtures fail (4 fixtures)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T03:04:48Z"
updated = "2026-10-15T11:13:37Z"
scope = ["crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/src/lint_facts.rs", "crates/cadetta-cli/tests/lint_entry_points.rs", "changelog.d/**"]
+++

Found while working ~X1PF271. solvers/{result-file-committed-fresh-not-run,s005-output-missing,s005-output-wrong-kind,solve-stale-result-runs} fail: stdout lacks 'fresh ...' / missing S005.
