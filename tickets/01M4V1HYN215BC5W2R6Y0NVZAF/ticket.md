+++
id = "01M4V1HYN215BC5W2R6Y0NVZAF"
title = "CLI obligation universe omits fit, require, ots and citation obligations"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:29:29Z"
updated = "2026-10-15T03:03:54Z"
scope = ["crates/cadetta-cli/src/**", "changelog.d/01M4V1HYN215BC5W2R6Y0NVZAF.*"]

[[acceptance]]
text = "A fit appears in cadetta obligations, can be signed off with ack, and goes stale when it changes"
bound = false

[[acceptance]]
text = "ots and citation components appear as obligations (already shipped; covered by existing universe tests)"
bound = false
+++

obligations.md 2.3: ack require:/ots: selectors match no obligation, so O003 (require sign-off) and S004 (citation drift) and O002 on ots never fire; Citation file_digest is populated by sema but never enters an obligation. found while working ~P3E2FR7
