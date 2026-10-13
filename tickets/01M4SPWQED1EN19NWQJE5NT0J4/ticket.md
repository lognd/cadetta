+++
id = "01M4SPWQED1EN19NWQJE5NT0J4"
title = "BLAKE3 own and closure Merkle digests"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4SPWGV64X8H1TN1YBR6K5CH"
reporter = "lognd"
created = "2026-10-13T04:03:53Z"
updated = "2026-10-13T07:41:24Z"
persona = "cadetta contributor"
capability = "compute canonical digests excluding spans and docs"
outcome_text = "waiver staleness and drift attribution are exact"
labels = ["creates:crates/cadetta-ir/src/digest.rs"]
scope = ["crates/cadetta-ir/src/digest.rs", "crates/cadetta-ir/Cargo.toml", "crates/cadetta-ir/src/lib.rs", "Cargo.lock", "docs/spec/ir.md"]

[[links]]
kind = "blocks"
target = "01M4SPWQH44JWFXZDNTGEBMMNX"

[[acceptance]]
text = "Given two sources differing only in spans, doc comments or formatting, when digested, then own and closure digests are equal (ir.md 4)"
bound = false

[[acceptance]]
text = "Given a changed value, when digested, then the node own digest changes (ir.md 4)"
bound = false

[[acceptance]]
text = "Given a node, when its closure is computed, then it is over its own digest plus the sorted closure digests of its deps, including cited file digests and solver result digests (ir.md 4)"
bound = false

[[acceptance]]
text = "Given an instance path such as Rover/wheel[2].hub_bore, when its closure is computed, then it includes only the part arguments the member reads, transitively (ir.md 4)"
bound = false

[[acceptance]]
text = "Given an interface-typed slot whose primary changes, when closures are recomputed, then every instance path of the slot changes and a check reading only another part arguments does not (ir.md 4)"
bound = false

[[acceptance]]
text = "Given a part node, when its own digest is computed, then primary, alternates and arguments are included (ir.md 4)"
bound = false

[[acceptance]]
text = "Given semantic attributes (process, role, rule roles, default), when digested, then they enter the own digest; display, emit, allow and lifecycle date text do not (ir.md 4)"
bound = false

[[acceptance]]
text = "Given an interface-typed slot, when an instance closure is computed, then it includes the part node's own digest so a swap stales that slot only (ir.md 4)"
bound = false

[[acceptance]]
text = "Given Datum and Placement nodes, when digested, then a moved datum changes the closure of everything that reads through it (ir.md 4; geometry.md 6)"
bound = false
+++
