+++
id = "01M4SWK92ASN873QYBKXGPTZV0"
title = "IR Dim vector over SI base, angle and currency"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T05:43:35Z"
updated = "2026-10-13T07:40:13Z"
persona = "design engineer"
capability = "build quantities from the IR alone"
idempotency_key = "replan14:ir-dim"
labels = ["creates:crates/cadetta-ir/src/dim.rs"]
scope = ["crates/cadetta-ir/src/dim.rs", "crates/cadetta-ir/Cargo.toml", "crates/cadetta-ir/src/lib.rs", "Cargo.lock", "crates/cadetta-units/src/dimension.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWHVJVHRD0KWP8ESJD81M"

[[links]]
kind = "blocked-by"
target = "01M4SPWQ3VSX2CA3TZY4KQ2DZW"

[[acceptance]]
text = "Given a Value, when serialized, then dim holds integer exponents for m, kg, s, A, K, mol, cd, angle and a currency map keyed by ISO code (ir.md 3)"
bound = true

[[acceptance]]
text = "Given an IR with no unit table, when a Quantity is built, then dim alone suffices to convert units (ir.md 3)"
bound = true
+++
