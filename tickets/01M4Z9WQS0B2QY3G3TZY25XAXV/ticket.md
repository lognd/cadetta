+++
id = "01M4Z9WQS0B2QY3G3TZY25XAXV"
title = "Conformance: geometry push-plan fixtures need a source_form to binding-target planner"
type = "task"
category = "in-progress"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-15T08:12:08Z"
updated = "2026-10-15T20:58:32Z"
scope = ["crates/cadetta-adapter/**", "crates/cadetta-cli/tests/**"]

[[acceptance]]
text = "Every geom-* push-plan fixture executes and is compared; their known-gaps.toml entries are removed."
bound = false
+++

found while working ~BC7S5KT: geom-* push-plan fixtures (binds_to, push_writes, read_reports, native_written/read_back_as) have no library decision function; negate_text is pub(crate) in cadetta-cli emit.rs.
