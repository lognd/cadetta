+++
id = "01M4Z7SF3KHPF3NA2YB3P4PXDX"
title = "cadetta ir: config lint.precision.Money rejected as unknown quantity kind (golden 07)"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-15T07:35:24Z"
updated = "2026-10-15T07:51:07Z"
scope = ["crates/cadetta-cli/src/config.rs", "docs/spec/lints.md", "changelog.d/01M4Z7SF3KHPF3NA2YB3P4PXDX.*"]

[[acceptance]]
text = "lint.precision.Money is accepted and sets the M001 limit for USD, EUR, GBP, CNY and JPY"
bound = true
+++

found while working ~C7DZBZH
