+++
id = "01M4SWK92ASN873QYBKXGPTZV0"
title = "IR Dim vector over SI base, angle and currency"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T05:43:35Z"
updated = "2026-10-13T05:43:37Z"
persona = "design engineer"
capability = "build quantities from the IR alone"
idempotency_key = "replan14:ir-dim"
labels = ["creates:crates/cadetta-ir/src/dim.rs"]
scope = ["crates/cadetta-ir/src/dim.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWHVJVHRD0KWP8ESJD81M"

[[acceptance]]
text = "Given a Value, when serialized, then dim holds integer exponents for m, kg, s, A, K, mol, cd, angle and a currency map keyed by ISO code (ir.md 3)"
bound = false

[[acceptance]]
text = "Given an IR with no unit table, when a Quantity is built, then dim alone suffices to convert units (ir.md 3)"
bound = false
+++
