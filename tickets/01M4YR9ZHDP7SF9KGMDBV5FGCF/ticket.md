+++
id = "01M4YR9ZHDP7SF9KGMDBV5FGCF"
title = "solvers: freshness stdout and S005 fixtures fail (4 fixtures)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T03:04:48Z"
updated = "2026-10-15T12:37:40Z"
scope = ["crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/src/lint_facts.rs", "crates/cadetta-cli/tests/lint_entry_points.rs", "changelog.d/**", "tests/conformance/known-gaps.toml", "tests/conformance/solvers/result-file-committed-fresh-not-run.expect.json", "tests/conformance/solvers/solve-stale-result-runs.expect.json"]

[[acceptance]]
text = "solvers freshness and stale-run fixtures pass under the conformance runner"
bound = true
+++

Found while working ~X1PF271. solvers/{result-file-committed-fresh-not-run,s005-output-missing,s005-output-wrong-kind,solve-stale-result-runs} fail: stdout lacks 'fresh ...' / missing S005.
