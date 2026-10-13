+++
id = "01M4SPWQED1EN19NWQJE5NT0J4"
title = "BLAKE3 digests of params and components"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T04:04:00Z"
persona = "cadetta contributor"
capability = "compute canonical digests excluding spans and docs"
outcome_text = "waiver staleness and drift attribution are exact"
labels = ["creates:crates/cadetta-ir/src/digest.rs"]
scope = ["crates/cadetta-ir/src/digest.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWQH44JWFXZDNTGEBMMNX"

[[acceptance]]
text = "Given two sources differing only in spans and doc comments, when digested, then digests are equal"
bound = false

[[acceptance]]
text = "Given a changed value, when digested, then the digest changes"
bound = false
+++
