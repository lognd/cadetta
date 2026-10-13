+++
id = "01M4ST8EMQTXAGZAWGE2M5A4BR"
title = "Obligation graph nodes, edges and states"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4ST8EJWTGQM9CMVBJVBATJT"
reporter = "lognd"
created = "2026-10-13T05:02:43Z"
updated = "2026-10-13T05:44:39Z"
persona = "design engineer"
capability = "see every claim as a node with typed edges"
idempotency_key = "replan13:Obligation graph nodes, edges and states"
labels = ["creates:crates/cadetta-obligations/src/graph.rs"]
scope = ["crates/cadetta-obligations/src/graph.rs"]

[[links]]
kind = "blocks"
target = "01M4ST8EPKBWRWHF51Y5NV3VK0"

[[acceptance]]
text = "Given a lowered IR, when the graph is built, then edges derives, cites, implements, instantiates, evaluates, solves, binds, realizes, attests and waives exist as in the obligations.md 1 table"
bound = false

[[acceptance]]
text = "Given the states discharged, stale, violated, undecided, open and waived, when an obligation is classified, then exactly one applies (obligations.md 2)"
bound = false

[[acceptance]]
text = "Given a check, require, fit or alternate-viability obligation, when classified, then it is recomputed every run and never stale (obligations.md 2)"
bound = false

[[acceptance]]
text = "Given an obligation, when identified, then its id is <kind>:<canonical path> and kinds include waiver (obligations.md 2, 4)"
bound = false
+++
