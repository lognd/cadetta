+++
id = "01M4Y6TC5EN0M1WMP9AK3YMAS8"
title = "KindChecker: test :: qualified paths through path_type"
type = "task"
category = "todo"
priority = "low"
points = 1
reporter = "lognd"
created = "2026-10-14T21:59:11Z"
updated = "2026-10-16T00:26:27Z"
idempotency_key = "kindcheck-qualified-path-tests"
scope = ["crates/cadetta-sema", "crates/cadetta-cli/tests"]

[[acceptance]]
text = "cadetta check reports Q002 for a read of a pkg::Mod::Comp.member path whose dimension mismatches the declared kind"
bound = false

[[acceptance]]
text = "cadetta check reports no Q or E diagnostic for a matching-dimension pkg::Mod::Comp.member read"
bound = false
+++

Follow-up to ~CH5HR2K. The new path_type method threads :: separators, and docs/spec/quantities.md 4.4 plus the changelog claim qualified paths resolve, but no test covers them. Add tests for pkg::Mod::Comp.width read with a wrong and a matching dimension through cadetta check.
