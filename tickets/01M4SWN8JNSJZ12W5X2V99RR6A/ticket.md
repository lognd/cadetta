+++
id = "01M4SWN8JNSJZ12W5X2V99RR6A"
title = "Scoping with --namespace and GLOB"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T05:44:40Z"
updated = "2026-10-13T05:44:45Z"
persona = "design engineer"
capability = "work on part of a project"
idempotency_key = "replan14:cli-scope"
labels = ["creates:crates/cadetta-cli/src/scope.rs"]
scope = ["crates/cadetta-cli/src/scope.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWN8BMEE0S29Y4N4EZTYJT"

[[acceptance]]
text = "Given check, emit, sync or solve with --namespace NS or canonical-path globs, when run, then only matching nodes are processed (config-cli.md 3)"
bound = false
+++
