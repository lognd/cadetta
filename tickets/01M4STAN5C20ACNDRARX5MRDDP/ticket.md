+++
id = "01M4STAN5C20ACNDRARX5MRDDP"
title = "Result files <stem>.out.json keyed by input digest"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STAMTNCTC1MRHE5VYP9QYN"
reporter = "lognd"
created = "2026-10-13T05:03:55Z"
updated = "2026-10-13T07:26:44Z"
persona = "design engineer"
capability = "keep solver results in the repo keyed by their inputs"
idempotency_key = "replan13:Result files keyed by input digest"
labels = ["creates:crates/cadetta-solvers/src/results.rs"]
scope = ["crates/cadetta-solvers/src/results.rs", "crates/cadetta-solve/src/results.rs", "crates/cadetta-solve/src/lib.rs", "crates/cadetta-solve/Cargo.toml", "Cargo.lock", "docs/spec/solvers.md"]

[[links]]
kind = "blocks"
target = "01M4STAN8QG3QWZQCH5XN3QHMT"

[[links]]
kind = "blocks"
target = "01M4STANBCA3XCNHR3X1NJKGFE"

[[links]]
kind = "blocks"
target = "01M4STANFNKA8G30C6KEEBM38C"

[[acceptance]]
text = "Given unchanged inputs, when evaluated, then the cached result is used and the solver is not run (solvers.md 4)"
bound = false

[[acceptance]]
text = "Given changed inputs, when evaluated, then the output is stale but still evaluates until re-solved (solvers.md 4)"
bound = false

[[acceptance]]
text = "Given a solve run, when stored, then `<stem>.out.json` beside the script (drive_torque.py writes drive_torque.out.json) has schema `cadetta.solve-result/1` with results[] of solver, input_digest, script_digest, dep_digests, version?, variant?, inputs, outputs and diagnostics, sorted by solver path then variant (solvers.md 4)"
bound = false

[[acceptance]]
text = "Given two solvers sharing one script, when stored, then results are keyed inside the one file by canonical solver path (acme.drive::DriveModule.drive_torque for a member, acme.drive::drive_torque at file scope) and never collide (solvers.md 4)"
bound = false

[[acceptance]]
text = "Given result digests, when recorded, then they live only in the committed .out.json and never in cadetta.lock (solvers.md 4; obligations.md 3)"
bound = false
+++
