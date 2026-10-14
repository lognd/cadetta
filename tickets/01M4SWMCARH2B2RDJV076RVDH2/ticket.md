+++
id = "01M4SWMCARH2B2RDJV076RVDH2"
title = "Host handling of R008, R009 and R010"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:44:11Z"
updated = "2026-10-14T21:07:44Z"
persona = "design engineer"
capability = "get clear errors when the CAD tool is unavailable"
idempotency_key = "replan14:ad-r8910"
labels = ["creates:crates/cadetta-adapter/src/availability.rs"]
scope = ["crates/cadetta-adapter/src/availability.rs", "crates/cadetta-adapter/src/lib.rs", "docs/spec/adapters.md", "changelog.d/*76RVDH2*"]

[[links]]
kind = "blocked-by"
target = "01M4SWMC5P00K0B1SZ9JPTJW8W"

[[acceptance]]
text = "Given an unavailable adapter, when sync runs, then R008 is an error and exit 2; when check runs it is info (adapters.md 4; config-cli.md 3)"
bound = true

[[acceptance]]
text = "Given a busy or modal CAD session, when pushing, then R009 fails the file without touching it (adapters.md 4)"
bound = true

[[acceptance]]
text = "Given a read-only or protected file, when pushing, then R010 is reported per file (adapters.md 4)"
bound = false
+++
