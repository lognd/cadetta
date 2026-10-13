+++
id = "01M4VB63M63PQVRKN0HDXQP567"
title = "Citation lints and obligations: S002, S004, S008, attest citation records"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T19:17:46Z"
updated = "2026-10-13T19:17:46Z"
persona = "design engineer"
capability = "see when a cited source is incomplete or has changed"
outcome_text = "nobody signs off values against a superseded datasheet"
idempotency_key = "d15:cite-lints"
scope = ["crates/cadetta-lint/src/rules/s.rs", "crates/cadetta-lint/src/rules/mod.rs", "crates/cadetta-lint/src/engine.rs", "crates/cadetta-obligations/src/**", "crates/cadetta-cli/src/cmd/ack.rs", "tests/conformance/lints/**", "tests/conformance/obligations/**"]

[[acceptance]]
text = "Given a cite missing a field its type requires, a url with neither file nor accessed, or an ots value whose only citation is a note, when linted, then S008; pseudo-citations never are (lints.md S008; language.md 3.9)"
bound = false

[[acceptance]]
text = "Given a cite whose file is missing, when linted, then S002 (lints.md)"
bound = false

[[acceptance]]
text = "Given cadetta ack ots:PATH --role source, when run, then one attest citation:<cite path> per reached cite item is recorded with the citation digest; a later field or file change is S004 and never O001 (obligations.md 2.3)"
bound = false
+++
