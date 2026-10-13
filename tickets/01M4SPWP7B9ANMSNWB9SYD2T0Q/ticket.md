+++
id = "01M4SPWP7B9ANMSNWB9SYD2T0Q"
title = "Spans and error recovery"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWG5A0FJNPKRPDDCJQZN8"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T06:31:52Z"
persona = "cadetta contributor"
capability = "attach file:line:col spans to every node and recover from errors"
outcome_text = "diagnostics point at source and one error does not hide the rest"
labels = ["creates:crates/cadetta-syntax/src/span.rs"]
scope = ["crates/cadetta-syntax/src/span.rs", "crates/cadetta-syntax/src/lib.rs"]

[[acceptance]]
text = "Given two syntax errors in one file, when parsed, then both are reported with spans"
bound = false

[[acceptance]]
text = "Given any AST node, when queried, then it has a file:line:col-line:col span"
bound = false
+++
