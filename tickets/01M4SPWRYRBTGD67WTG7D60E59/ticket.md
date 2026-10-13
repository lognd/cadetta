+++
id = "01M4SPWRYRBTGD67WTG7D60E59"
title = "Bindings files and default name mangling"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T04:03:54Z"
updated = "2026-10-13T04:04:02Z"
persona = "design engineer"
capability = "get bindings/<adapter>.toml and deterministic default names"
outcome_text = "most bindings never need to be written"
labels = ["creates:crates/cadetta-adapter/src/bindings.rs"]
scope = ["crates/cadetta-adapter/src/bindings.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWSABPXKK62B5V0SMJ8FD"

[[acceptance]]
text = "Given `Axle.end_diameter` and a flat-namespace tool, when mangled, then the name is `Axle_end_diameter`"
bound = false

[[acceptance]]
text = "Given a parameter with `was`, when renamed, then its binding carries forward"
bound = false

[[acceptance]]
text = 'Given @native(adapter, "locator"), when loaded, then it overrides the file'
bound = false
+++
