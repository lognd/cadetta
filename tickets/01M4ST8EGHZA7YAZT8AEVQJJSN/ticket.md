+++
id = "01M4ST8EGHZA7YAZT8AEVQJJSN"
title = "Lifecycle attribute and replacement candidates"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4ST8E5XX0TX0MBND07JQME1"
reporter = "lognd"
created = "2026-10-13T05:02:43Z"
updated = "2026-10-13T05:05:24Z"
persona = "design engineer"
capability = "see when a primary reaches end of life and which alternate can replace it"
idempotency_key = "replan13:Lifecycle attribute and replacement candidates"
labels = ["creates:crates/cadetta-variants/src/lifecycle.rs"]
scope = ["crates/cadetta-variants/src/lifecycle.rs"]

[[links]]
kind = "blocks"
target = "01M4STCCQD6C55WEN7HFEBPEYV"

[[acceptance]]
text = 'Given `#[lifecycle(eol, "2027-06")]`, when parsed, then status active|nrnd|eol|obsolete and the date are recorded (language.md 3.2)'
bound = false

[[acceptance]]
text = "Given a primary with status eol or obsolete and no viable alternate, when analysed, then the L001 fact is recorded, error once its date has passed (lints.md L)"
bound = false

[[acceptance]]
text = "Given a primary with status nrnd, when analysed, then the L002 fact is recorded (lints.md L)"
bound = false

[[acceptance]]
text = "Given status active, when recorded, then the date means last verified (language.md 3.2)"
bound = false
+++
