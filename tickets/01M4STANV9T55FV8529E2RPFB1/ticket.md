+++
id = "01M4STANV9T55FV8529E2RPFB1"
title = "std packaging as ordinary .cdt"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:56Z"
updated = "2026-10-13T06:51:38Z"
persona = "design engineer"
capability = "import std modules like any other package"
idempotency_key = "replan13:std packaging as ordinary .cdt"
labels = ["creates:std/cadetta.toml", "creates:std/src/lib.cdt"]
scope = ["std/cadetta.toml", "std/src/lib.cdt", "crates/cadetta-cli/src/pkg/manifest.rs"]

[[links]]
kind = "blocks"
target = "01M4STANXPC2GK2PQN8E8NJG6J"

[[links]]
kind = "blocks"
target = "01M4STAP0CCQTR0MM7TAMWC9E8"

[[links]]
kind = "blocks"
target = "01M4STAP388RZR0PTDBKBV6JM5"

[[links]]
kind = "blocks"
target = "01M4STAPK7FN8DM11MV7GWXE3Z"

[[acceptance]]
text = "Given the version, when read, then it is 0.1.0 (stdlib.md 1)"
bound = true

[[acceptance]]
text = "Given `use std::fits`, when resolved, then std is found as an ordinary package (std/cadetta.toml with [package], root std/src/lib.cdt) with no special-casing, and no user package may be named std (stdlib.md 1; packages.md 1)"
bound = true
+++
