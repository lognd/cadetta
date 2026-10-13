+++
id = "01M4SPWQH44JWFXZDNTGEBMMNX"
title = "Lower evaluated program to design IR"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T04:04:02Z"
persona = "cadetta contributor"
capability = "lower sema output into design IR with provenance, deps and spans"
outcome_text = "lints and emitters consume a single structure"
labels = ["creates:crates/cadetta-ir/src/lower.rs"]
scope = ["crates/cadetta-ir/src/lower.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWQM289M7Q58CSS6F029F"

[[links]]
kind = "blocks"
target = "01M4SPWRGXS8XKJCF2H8FDKPKC"

[[acceptance]]
text = "Given a derived parameter, when lowered, then provenance=derived and deps list its canonical paths"
bound = false

[[acceptance]]
text = "Given the language.md example, when lowered, then fits, checks and waivers appear in the Project"
bound = false
+++
