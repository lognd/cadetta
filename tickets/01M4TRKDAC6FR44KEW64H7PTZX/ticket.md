+++
id = "01M4TRKDAC6FR44KEW64H7PTZX"
title = "Lint lifts L/T/P/Q/E from sema and variants facts in check"
type = "task"
category = "todo"
priority = "medium"
points = 8
reporter = "lognd"
created = "2026-10-13T13:52:59Z"
updated = "2026-10-15T02:06:10Z"
labels = ["creates:crates/cadetta-cli/tests/lint_lifts.rs", "creates:changelog.d/01M4TRKDAC6FR44KEW64H7PTZX.added.md"]
scope = ["crates/cadetta-cli/Cargo.toml", "crates/cadetta-cli/src/cmd/check.rs", "crates/cadetta-cli/tests/lint_lifts.rs", "docs/spec/lints.md", "changelog.d/01M4TRKDAC6FR44KEW64H7PTZX.added.md", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-sema/src/**", "crates/cadetta-variants/src/**", "crates/cadetta-lint/src/**"]

[[acceptance]]
text = "L, T, F and Q lifts are fed real findings from sema and variants and appear in cadetta check output"
bound = true
+++

Follow-up of ~MC5WD33: lifecycle (L), not-in-table/missing-role (T), fit F001-F003 (P/F lifts) and Q finding lifts have no producing facts reaching the driver (lift_lifecycle, lift_not_in_table, lift_missing_role, f::lift, q::lift, e::lift_resolution). Produce the findings in sema/variants and call the lifts.
