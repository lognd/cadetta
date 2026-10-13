+++
id = "01M4STANFNKA8G30C6KEEBM38C"
title = "Solver lints S003, S005 and S006"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4STAMTNCTC1MRHE5VYP9QYN"
reporter = "lognd"
created = "2026-10-13T05:03:55Z"
updated = "2026-10-13T07:53:32Z"
persona = "design engineer"
capability = "be told when solver results are stale or unused"
idempotency_key = "replan13:Solver lints S003, S005 and S006"
labels = ["creates:crates/cadetta-solvers/src/lints.rs"]
scope = ["crates/cadetta-solvers/src/lints.rs", "crates/cadetta-solve/src/lints.rs", "crates/cadetta-solve/src/lib.rs", "crates/cadetta-solve/Cargo.toml", "docs/spec/solvers.md", "Cargo.lock"]

[[links]]
kind = "blocks"
target = "01M4STCFE8P4ABFFCAFEX8DC9C"

[[acceptance]]
text = "Given a stale result, when linted, then S003 is raised (lints.md S)"
bound = true

[[acceptance]]
text = "Given the S005 and S006 conditions in lints.md, when linted, then each fires and has a non-firing case (lints.md S)"
bound = true
+++
