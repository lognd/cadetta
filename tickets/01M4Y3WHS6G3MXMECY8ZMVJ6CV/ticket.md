+++
id = "01M4Y3WHS6G3MXMECY8ZMVJ6CV"
title = "Wire availability (R008-R010) into the sync and check commands"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 3
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-14T21:07:56Z"
updated = "2026-10-15T10:12:40Z"
persona = "design engineer"
capability = "get clear errors when the CAD tool is unavailable"
scope = ["crates/cadetta-cli/src/cmd/sync.rs", "crates/cadetta-adapter/src/session.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWMCARH2B2RDJV076RVDH2"

[[acceptance]]
text = "Given an unavailable adapter, when cadetta sync runs, then the process exits 2 with R008"
bound = true

[[acceptance]]
text = "Given a dirty file among several, when sync runs, then only that file is refused (R009) and the rest are pushed"
bound = true

[[acceptance]]
text = "Given rejected locators, when sync runs, then one R010 per file appears in sync output"
bound = true
+++

cadetta_adapter::availability (T ~76RVDH2) is not yet called. Make sync use unavailable() for exit 2 and check for info plus lock base; make DriveSession::open use partition_busy so other files proceed; surface rejections() as R010 in sync output. Note: ticket criterion 3 says read-only is R010 but spec adapters.md 2.3 says R009; the module follows the spec (read-only is R009, adapter-rejected locators R010).
