+++
id = "01M4YEA3QBT5257Z8BAN7NH9KY"
title = "Lower interface bounds into lint_facts::interfaces"
type = "task"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T00:10:06Z"
updated = "2026-10-15T09:14:18Z"
scope = ["crates/cadetta-cli/src/lint_facts.rs"]

[[links]]
kind = "blocked-by"
target = "01M4TA3CABXHEZFA7BXA8J3XJW"

[[acceptance]]
text = "An interface bound violation is reported through check"
bound = false
+++

Follow-up of ~A8J3XJW. lint_facts::interfaces leaves bounds empty because the IR Project has no interfaces field; lower bounds from the AST so bounds checks see them.
