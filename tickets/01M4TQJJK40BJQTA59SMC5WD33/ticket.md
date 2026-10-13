+++
id = "01M4TQJJK40BJQTA59SMC5WD33"
title = "Wire lint rule families (D, F, L, M, N, S, T, P, W, Q, O) into the check driver"
type = "task"
category = "in-progress"
priority = "medium"
points = 8
reporter = "lognd"
created = "2026-10-13T13:35:03Z"
updated = "2026-10-13T13:43:37Z"
idempotency_key = "wire-lints"
scope = ["crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/src/cmd/check.rs", "crates/cadetta-lint/src/**", "docs/spec/lints.md"]

[[acceptance]]
text = "cadetta check runs every implemented lint family over the analyzed IR and reports their diagnostics"
bound = false

[[acceptance]]
text = "L/T/P/Q/N004 lifts are fed from sema and variants facts"
bound = false

[[acceptance]]
text = "waivers and allows are applied through the engine, W rules reported"
bound = false
+++

Rule modules exist as standalone APIs and are only unit-tested; nothing in the CLI calls them (see agents' reports on ~FEBPEYV, ~F4PFYCT, ~Q41GK55, ~ZSG20Z9, ~E0Q9EBH).
