+++
id = "01M4STAN28E01T9BY5M07ZD4V5"
title = "Solver transports and command template"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STAMTNCTC1MRHE5VYP9QYN"
reporter = "lognd"
created = "2026-10-13T05:03:55Z"
updated = "2026-10-13T07:18:14Z"
persona = "design engineer"
capability = "run solvers over stdio or files with a configured command"
idempotency_key = "replan13:Solver transports and command template"
labels = ["creates:crates/cadetta-solvers/src/transport.rs"]
scope = ["crates/cadetta-solvers/src/transport.rs", "crates/cadetta-solve/src/transport.rs", "crates/cadetta-solve/src/lib.rs", "docs/spec/solvers.md", "docs/spec/config-cli.md"]

[[links]]
kind = "blocks"
target = "01M4STAN8QG3QWZQCH5XN3QHMT"

[[acceptance]]
text = "Given `[solvers.NAME]` with a command template, when run, then placeholders are expanded and stdio or file transport is used (solvers.md 5, config-cli.md)"
bound = false

[[acceptance]]
text = "Given a nonzero exit or timeout, when run, then a typed error names the solver (solvers.md 3)"
bound = false

[[acceptance]]
text = "Given an unknown solver name, when run, then exit 2 with a message (config-cli.md 2)"
bound = false
+++
