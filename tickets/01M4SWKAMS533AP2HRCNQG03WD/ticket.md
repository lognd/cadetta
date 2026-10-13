+++
id = "01M4SWKAMS533AP2HRCNQG03WD"
title = "Lints O004 and W005"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SPWH0ZE4G7KXRDA5M71GPS"
reporter = "lognd"
created = "2026-10-13T05:43:36Z"
updated = "2026-10-13T05:43:39Z"
persona = "design engineer"
capability = "see expiries before they bite"
idempotency_key = "replan14:lint-o4"
labels = ["creates:crates/cadetta-lint/src/rules/o004_w005.rs", "creates:tests/lints/O004/**", "creates:tests/lints/W005/**"]
scope = ["crates/cadetta-lint/src/rules/o004_w005.rs", "tests/lints/O004/**", "tests/lints/W005/**"]

[[links]]
kind = "blocked-by"
target = "01M4SPWQM289M7Q58CSS6F029F"

[[links]]
kind = "blocked-by"
target = "01M4STCCZ58C875BTMQ9E54BMT"

[[acceptance]]
text = "Given a stale sync obligation, when linted, then O004 is info by default and gated by [obligations.kinds.sync] required_for (lints.md 3 O; obligations.md 2.3)"
bound = false

[[acceptance]]
text = "Given a waiver or eol date within expiry_warn_days (default 30) of --as-of, when linted, then W005 info fires; allow is excluded (lints.md 2.1, 2.2)"
bound = false
+++
