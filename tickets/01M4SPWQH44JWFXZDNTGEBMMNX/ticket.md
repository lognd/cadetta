+++
id = "01M4SPWQH44JWFXZDNTGEBMMNX"
title = "Lower evaluated program to design IR"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T08:54:37Z"
persona = "cadetta contributor"
capability = "lower sema output into design IR with provenance, deps and spans"
outcome_text = "lints and emitters consume a single structure"
labels = ["creates:crates/cadetta-ir/src/lower.rs", "creates:crates/cadetta-sema/src/lower.rs"]
scope = ["crates/cadetta-ir/src/lower.rs", "crates/cadetta-sema/src/lib.rs", "docs/spec/ir.md", "crates/cadetta-sema/src/lower.rs", "crates/cadetta-sema/src/eval.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWQM289M7Q58CSS6F029F"

[[links]]
kind = "blocks"
target = "01M4SPWRGXS8XKJCF2H8FDKPKC"

[[links]]
kind = "blocks"
target = "01M4SPWT3FP4SN4RTCHE631Y52"

[[links]]
kind = "blocks"
target = "01M4STCEQRY59W2Q6BGZ6YE6N5"

[[acceptance]]
text = "Given a derived parameter, when lowered, then provenance=derived and deps list its canonical paths"
bound = true

[[acceptance]]
text = "Given the language.md example, when lowered, then fits, checks and waivers appear in the Project"
bound = false

[[acceptance]]
text = "Given a `let`, when lowered, then the member has analysis=true and is never bound or emitted (ir.md 3)"
bound = false

[[acceptance]]
text = 'Given a variant evaluation, when lowered, then each member stores only the values differing from the primary under "slot=alternate" (ir.md 3, language.md 3.2)'
bound = false

[[acceptance]]
text = "Given a process-compensated member, when lowered, then `designed` holds the value before compensation and `nominal` the drawn value (ir.md 3, language.md 3.8)"
bound = false

[[acceptance]]
text = "Given a literal tolerance and a derived affine form, when lowered, then Value.tol holds an Affine of source-variable terms (ir.md 3)"
bound = false
+++
