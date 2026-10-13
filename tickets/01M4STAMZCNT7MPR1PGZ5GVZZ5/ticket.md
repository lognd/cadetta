+++
id = "01M4STAMZCNT7MPR1PGZ5GVZZ5"
title = "cadetta.solve/1 request and response protocol"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M4STAMTNCTC1MRHE5VYP9QYN"
reporter = "lognd"
created = "2026-10-13T05:03:55Z"
updated = "2026-10-13T07:14:40Z"
persona = "design engineer"
capability = "exchange inputs and outputs with a solver in a defined format"
idempotency_key = "replan13:cadetta.solve/1 request and response protocol"
labels = ["creates:crates/cadetta-solvers/src/protocol.rs"]
scope = ["crates/cadetta-solvers/src/protocol.rs", "crates/cadetta-solve/src/protocol.rs", "crates/cadetta-solve/src/lib.rs", "crates/cadetta-solve/Cargo.toml", "docs/spec/solvers.md"]

[[links]]
kind = "blocks"
target = "01M4STAN28E01T9BY5M07ZD4V5"

[[links]]
kind = "blocks"
target = "01M4STAN5C20ACNDRARX5MRDDP"

[[acceptance]]
text = "Given a solver run, when the request is built, then it is `cadetta.solve/1` JSON with inputs as lit or si values (solvers.md 3)"
bound = true

[[acceptance]]
text = "Given a response, when parsed, then outputs must be literals; an expression output raises Q001 (solvers.md 3)"
bound = true

[[acceptance]]
text = "Given a malformed or wrong-version response, when parsed, then a typed error is returned (solvers.md 3)"
bound = true
+++
