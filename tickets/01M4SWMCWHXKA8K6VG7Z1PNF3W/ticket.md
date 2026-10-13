+++
id = "01M4SWMCWHXKA8K6VG7Z1PNF3W"
title = "Import --update and --write"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:44:11Z"
updated = "2026-10-13T05:44:14Z"
persona = "design engineer"
capability = "pull CAD edits back into source"
idempotency_key = "replan14:ad-imp-upd"
labels = ["creates:crates/cadetta-adapter/src/import/update.rs"]
scope = ["crates/cadetta-adapter/src/import/update.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWMCT03T7KB7HZRJXRXM2Y"

[[acceptance]]
text = "Given CAD values changed since adoption, when import --update runs, then a diff of .cdt literal edits is printed; only literals are edited, expressions never (adapters.md 7.4)"
bound = false

[[acceptance]]
text = "Given --write, when run, then sources are rewritten and the lock updated; otherwise nothing is written (config-cli.md 3)"
bound = false
+++
