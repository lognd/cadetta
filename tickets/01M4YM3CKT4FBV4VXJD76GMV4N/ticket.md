+++
id = "01M4YM3CKT4FBV4VXJD76GMV4N"
title = "Lint precision config accepts custom quantity kinds"
type = "story"
category = "in-progress"
priority = "low"
points = 3
reporter = "lognd"
created = "2026-10-15T01:51:18Z"
updated = "2026-10-16T00:52:38Z"
idempotency_key = "followup-xn48psv-custom-kinds"
scope = ["crates/cadetta-cli/src/config.rs", "crates/cadetta-cli/src/pipeline.rs", "docs/spec/lints.md"]

[[acceptance]]
text = "A precision entry naming a package-declared kind is accepted and changes M001; unknown kinds still rejected, with tests"
bound = true
+++

[lint] precision only consults the built-in kind table; custom quantity kinds declared in a package are rejected at config load. Resolve kinds from the package quantity table too.
