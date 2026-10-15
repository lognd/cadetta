+++
id = "01M4YEA3NHSS5JTGZRCCQ1ETZT"
title = "I001: report a member present with a different kind"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T00:10:06Z"
updated = "2026-10-15T08:38:06Z"
scope = ["crates/cadetta-lint/src/rules/interface.rs"]

[[links]]
kind = "blocked-by"
target = "01M4TA3CABXHEZFA7BXA8J3XJW"

[[acceptance]]
text = "I001 fires through check when the implementer has the member under a different kind"
bound = false
+++

Follow-up of ~A8J3XJW. check_members only tests presence by name; language.md 3.2 requires a same-name member of a different kind to raise I001. Fix in rules::interface and test through check.
