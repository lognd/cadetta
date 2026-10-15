+++
id = "01M4W5J6FACWMX38X4FTGY21E7"
title = "Package [defaults] and dependency packages in the evaluator inputs"
type = "task"
category = "in-progress"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-14T02:58:45Z"
updated = "2026-10-15T09:14:46Z"
scope = ["crates/cadetta-sema/**", "tests/conformance/known-gaps.toml", "tests/conformance/stdlib/**", "crates/cadetta-cli/src/pipeline.rs", "docs/**", "crates/cadetta-cli/src/cmd/solve.rs"]

[[acceptance]]
text = "Given [defaults] FastenerPolicy in cadetta.toml and a path dependency, when cadetta check runs, then the policy-default-* fixtures pass"
bound = true
+++

found while working ~646D1T7: PackageSrc has no defaults; the pipeline does not pass dependency packages (unknown package acme.shop). Needs crates/cadetta-cli/src/pipeline.rs scope.
