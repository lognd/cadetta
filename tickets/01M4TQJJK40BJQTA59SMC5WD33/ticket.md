+++
id = "01M4TQJJK40BJQTA59SMC5WD33"
title = "Wire lint rule families (D, F, L, M, N, S, T, P, W, Q, O) into the check driver"
type = "task"
category = "in-progress"
priority = "medium"
points = 8
reporter = "lognd"
created = "2026-10-13T13:35:03Z"
updated = "2026-10-13T13:53:07Z"
idempotency_key = "wire-lints"
scope = ["crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/src/cmd/check.rs", "crates/cadetta-lint/src/**", "docs/spec/lints.md"]

[[acceptance]]
text = "cadetta check runs the IR-only lint families (D001, D002, F004, M001-M003, N001-N003, N005, N006, Q006, S001, S002) over the analyzed IR and reports their diagnostics"
bound = true

[[acceptance]]
text = "waive declarations are collected from the in-scope modules and applied through the lint engine; W001-W004 are reported"
bound = true
+++

Rule modules exist as standalone APIs and are only unit-tested; nothing in the CLI calls them (see agents' reports on ~FEBPEYV, ~F4PFYCT, ~Q41GK55, ~ZSG20Z9, ~E0Q9EBH).
