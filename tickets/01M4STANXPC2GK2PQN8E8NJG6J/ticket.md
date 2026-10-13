+++
id = "01M4STANXPC2GK2PQN8E8NJG6J"
title = "std.fits (ISO 286)"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:03:56Z"
updated = "2026-10-13T07:10:10Z"
persona = "design engineer"
capability = "use standard fit classes"
idempotency_key = "replan13:std.fits (ISO 286)"
labels = ["creates:std/src/fits.cdt"]
scope = ["std/src/fits.cdt", "crates/cadetta-fits/tests/std_fits.rs", "crates/cadetta-fits/Cargo.toml", "docs/spec/stdlib.md", "crates/cadetta-syntax/tests/std_fits.rs"]

[[links]]
kind = "blocks"
target = "01M4STAPPJR0478N5B1E751RZY"

[[acceptance]]
text = "Given the iso286 fit classes in stdlib.md 2, when evaluated, then deviations match the published ISO 286 values, each cited (stdlib.md 1)"
bound = true

[[acceptance]]
text = "Given a bore and shaft, when a fit is evaluated, then limits resolve to the expected clearance (stdlib.md 1)"
bound = true
+++
