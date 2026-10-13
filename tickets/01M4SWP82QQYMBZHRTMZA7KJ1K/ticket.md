+++
id = "01M4SWP82QQYMBZHRTMZA7KJ1K"
title = "std.commercial::Sourced"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:45:12Z"
updated = "2026-10-13T07:43:41Z"
persona = "design engineer"
capability = "record where parts come from"
idempotency_key = "replan14:sd-sourced"
labels = ["creates:std/src/commercial.cdt"]
scope = ["std/src/commercial.cdt", "crates/cadetta-syntax/tests/std_commercial.rs"]

[[links]]
kind = "blocked-by"
target = "01M4STANV9T55FV8529E2RPFB1"

[[acceptance]]
text = "Given Sourced, when a component uses it, then supplier, part number and price members are optional and non-emitting (stdlib.md 6)"
bound = false
+++
