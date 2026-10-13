+++
id = "01M4SPWHPHR3VVX5NCSXF4502V"
title = "Cargo workspace skeleton with all crates"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 3
parent = "01M4SPWFRR9DAEM60DX796GVPN"
reporter = "lognd"
created = "2026-10-13T04:03:47Z"
updated = "2026-10-13T06:08:25Z"
persona = "cadetta contributor"
capability = "build a Cargo workspace containing cadetta-syntax, -units, -fits, -sema, -ir, -lint, -adapter, -cli and adapters/freecad"
outcome_text = "every epic has a crate to land code in"
labels = ["creates:Cargo.toml", "creates:crates/*/Cargo.toml", "creates:crates/*/src/lib.rs", "creates:adapters/freecad/pyproject.toml", "creates:crates/cadetta-geometry/Cargo.toml", "creates:crates/cadetta-geometry/src/lib.rs"]
scope = ["Cargo.toml", "crates/*/Cargo.toml", "crates/*/src/lib.rs", "adapters/freecad/pyproject.toml", "crates/cadetta-geometry/Cargo.toml", "crates/cadetta-geometry/src/lib.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWHRZF9R1KGA3GV2AZF30"

[[links]]
kind = "blocks"
target = "01M4SPWNYX632H3Q6WEGN0SM0R"

[[links]]
kind = "blocks"
target = "01M4SPWSTRG1FFAJ0XM89ETBBN"

[[acceptance]]
text = "Given the workspace, when `cargo test --workspace` runs, then it passes with zero tests failing"
bound = true

[[acceptance]]
text = "Given the draft 1.4 crate list, when the workspace builds, then crates cadetta-geometry, cadetta-variants, cadetta-obligations and cadetta-solvers are members too (geometry.md; ir.md)"
bound = true

[[acceptance]]
text = "Given a clean checkout, when cargo build --workspace runs, then every workspace crate compiles"
bound = true
+++
