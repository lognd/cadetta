+++
id = "01M4Y772CY1967MXZ9T5T8FKPM"
title = "Derive file-scope fit items (ItemKind::Fit) into lowering"
type = "task"
category = "in-progress"
priority = "low"
points = 2
reporter = "lognd"
created = "2026-10-14T22:06:07Z"
updated = "2026-10-16T01:44:24Z"
scope = ["crates/cadetta-sema/src/fitderive.rs", "crates/cadetta-sema/src/lower.rs"]

[[acceptance]]
text = "A file-scope fit item is evaluated and lowered into IR (location decided by the spec), with a test"
bound = false
+++

Split from ~Q1ZBN9Y. ast.rs has a file-scope ItemKind::Fit (ast.rs:274/452) that fitderive does not handle; only MemberKind::Fit inside components is derived.
