+++
id = "01M4SPWHPHR3VVX5NCSXF4502V"
title = "Cargo workspace skeleton with all crates"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWFRR9DAEM60DX796GVPN"
reporter = "lognd"
created = "2026-10-13T04:03:47Z"
updated = "2026-10-13T04:03:47Z"
persona = "cadetta contributor"
capability = "build a Cargo workspace containing cadetta-syntax, -units, -fits, -sema, -ir, -lint, -adapter, -cli and adapters/freecad"
outcome_text = "every epic has a crate to land code in"
labels = ["creates:Cargo.toml", "creates:crates/*/Cargo.toml", "creates:crates/*/src/lib.rs", "creates:adapters/freecad/pyproject.toml"]
scope = ["Cargo.toml", "crates/*/Cargo.toml", "crates/*/src/lib.rs", "adapters/freecad/pyproject.toml"]

[[acceptance]]
text = "Given a clean checkout, when `cargo build --workspace` runs, then all eight crates compile"
bound = false

[[acceptance]]
text = "Given the workspace, when `cargo test --workspace` runs, then it passes with zero tests failing"
bound = false
+++
