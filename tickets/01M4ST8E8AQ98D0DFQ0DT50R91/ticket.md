+++
id = "01M4ST8E8AQ98D0DFQ0DT50R91"
title = "Interface declarations and implementation checking"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4ST8E5XX0TX0MBND07JQME1"
reporter = "lognd"
created = "2026-10-13T05:02:42Z"
updated = "2026-10-13T07:26:02Z"
persona = "design engineer"
capability = "declare what a design needs from a part and have implementers checked against it"
idempotency_key = "replan13:Interface declarations and implementation checking"
labels = ["creates:crates/cadetta-variants/src/interface.rs"]
scope = ["crates/cadetta-variants/src/interface.rs", "crates/cadetta-variants/src/lib.rs", "crates/cadetta-variants/Cargo.toml", "Cargo.lock", "docs/spec/language.md"]

[[links]]
kind = "blocks"
target = "01M4ST8EABK1PHDV3MGSY9GBF1"

[[links]]
kind = "blocks"
target = "01M4ST8EGHZA7YAZT8AEVQJJSN"

[[acceptance]]
text = "Given `interface Caster { diameter: Length  load_rating >= 400N }`, when resolved, then members carry kinds and bounds (language.md 3.2)"
bound = true

[[acceptance]]
text = "Given a component `: Caster` lacking a member, when checked, then I001 is raised and cannot be waived (language.md 3.2, lints.md I)"
bound = true

[[acceptance]]
text = "Given an implementer violating an interface bound, when checked, then C001 is raised against that implementer (language.md 3.2)"
bound = true

[[acceptance]]
text = "Given code reading `wheel.hub_bore`, when resolved, then it depends on the interface only (language.md 3.2)"
bound = true
+++
