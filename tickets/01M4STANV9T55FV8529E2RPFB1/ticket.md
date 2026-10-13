+++
id = "01M4STANV9T55FV8529E2RPFB1"
title = "std packaging as ordinary .cdt"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:56Z"
updated = "2026-10-13T05:03:56Z"
persona = "design engineer"
capability = "import std modules like any other package"
idempotency_key = "replan13:std packaging as ordinary .cdt"
labels = ["creates:std/cadetta.toml", "creates:std/mod.cdt"]
scope = ["std/cadetta.toml", "std/mod.cdt"]

[[acceptance]]
text = "Given `use std.fits`, when resolved, then std is found as an ordinary package with no special-casing (stdlib.md 1)"
bound = false

[[acceptance]]
text = "Given the version, when read, then it is 0.1.0 (stdlib.md 1)"
bound = false
+++
