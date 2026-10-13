+++
id = "01M4SXRFQ0RENW7YM154W0EWP9"
title = "cadetta update"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SXQPADRSNTEY3J7PH899Y8"
reporter = "lognd"
created = "2026-10-13T06:03:54Z"
updated = "2026-10-13T09:46:26Z"
persona = "product engineer"
capability = "re-resolve dependencies on demand"
labels = ["creates:crates/cadetta-cli/src/cmd/update.rs"]
scope = ["crates/cadetta-cli/src/cmd/update.rs", "docs/schemas/update.json", "docs/spec/packages.md", "docs/spec/config-cli.md", "crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-cli/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWSXBTNXW3CPZ6FVHT5ZT"

[[links]]
kind = "blocked-by"
target = "01M4SXR2FQZ6DZDY01VF34J1C5"

[[links]]
kind = "blocked-by"
target = "01M4SXR720K3VGM1K7DWZ0VJ7Z"

[[acceptance]]
text = "Given `cadetta update`, when run, then every dependency is re-resolved and its package record rewritten in cadetta.lock; `update NAME` re-resolves only NAME (packages.md 1; config-cli.md 3)"
bound = true

[[acceptance]]
text = "Given a conflict or unmet version, when run, then E012 or E013 gives exit 1 and the lock is unchanged (config-cli.md 3)"
bound = false

[[acceptance]]
text = "Given a run, when finished, then update.json is emitted and registered in the output schemas (config-cli.md 3)"
bound = false
+++
