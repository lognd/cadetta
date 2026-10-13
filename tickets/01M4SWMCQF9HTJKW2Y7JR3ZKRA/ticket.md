+++
id = "01M4SWMCQF9HTJKW2Y7JR3ZKRA"
title = "Import naming of members and files"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:44:11Z"
updated = "2026-10-13T13:21:56Z"
persona = "design engineer"
capability = "get readable imported names"
idempotency_key = "replan14:ad-imp-name"
labels = ["creates:crates/cadetta-adapter/src/import/naming.rs"]
scope = ["crates/cadetta-adapter/src/import/naming.rs", "crates/cadetta-adapter/src/import/mod.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWMCMY3QH9282ZJ07XBBB7"

[[acceptance]]
text = "Given native names, when scaffolded, then Cadetta member names, components and cad/imported/<File>.cdt follow adapters.md 7.2 deterministically"
bound = true
+++
