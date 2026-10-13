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

[[links]]
kind = "blocks"
target = "01M4SPWG5A0FJNPKRPDDCJQZN8"

[[links]]
kind = "blocks"
target = "01M4SPWGB2729B40CQE81MHM2Z"

[[links]]
kind = "blocks"
target = "01M4SPWGV64X8H1TN1YBR6K5CH"

[[links]]
kind = "blocks"
target = "01M4SPWH0ZE4G7KXRDA5M71GPS"
+++

Outcome: a Cargo workspace with every crate, CI and frob check gates so all other epics can land code.

Success metric: cargo build, cargo test and frob check exit 0 in CI on an empty workspace.
