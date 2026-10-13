+++
id = "01M4SPSW297R88BJJ2YJZFWB46"
title = "Workspace bootstrap"
type = "epic"
category = "done"
outcome = "wont-fix"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T04:02:19Z"
updated = "2026-10-13T05:00:00Z"
labels = ["creates:Cargo.toml"]
scope = ["Cargo.toml"]
+++

Outcome: a Cargo workspace with all crates, CI and frob check gates so every other epic can land code.
Success metric: cargo build and cargo test pass on an empty workspace in CI, and frob check exits 0 on main.
