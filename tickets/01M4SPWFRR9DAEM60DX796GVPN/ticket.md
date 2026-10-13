+++
id = "01M4SPWFRR9DAEM60DX796GVPN"
title = "Workspace bootstrap"
type = "epic"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T04:03:45Z"
updated = "2026-10-13T04:04:05Z"
labels = ["creates:Cargo.toml"]
scope = ["Cargo.toml"]

[[links]]
kind = "blocks"
target = "01M4SPWFZP4041AN6YQE08X2AN"
+++

Outcome: a Cargo workspace with every crate, CI and frob check gates so all other epics can land code.

Success metric: cargo build, cargo test and frob check exit 0 in CI on an empty workspace.
