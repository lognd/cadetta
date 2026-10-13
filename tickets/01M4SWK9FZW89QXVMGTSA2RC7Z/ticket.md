+++
id = "01M4SWK9FZW89QXVMGTSA2RC7Z"
title = "Variant naming SLOT=ALT in short and full forms"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T05:43:35Z"
updated = "2026-10-13T08:20:14Z"
persona = "design engineer"
capability = "name a variant once everywhere"
idempotency_key = "replan14:ir-variant"
labels = ["creates:crates/cadetta-ir/src/variant_name.rs"]
scope = ["crates/cadetta-ir/src/variant_name.rs", "docs/spec/ir.md"]

[[links]]
kind = "blocked-by"
target = "01M4SWK9D2DYTDN1TW3013FBDV"

[[acceptance]]
text = "Given Rover/wheel=acme.rover::Colson2Series, when parsed, then slot instance path and alternate component path are separated (ir.md 2)"
bound = true

[[acceptance]]
text = "Given the short form wheel=Colson2Series inside source and diagnostics, when resolved, then it is unambiguous; IR keys and lock records always use the full form (ir.md 2)"
bound = true
+++
