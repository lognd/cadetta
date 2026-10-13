+++
id = "01M4SWN8DY2SQYR5GCTQQM16Z9"
title = "cite pin"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T05:44:40Z"
updated = "2026-10-13T07:03:22Z"
persona = "design engineer"
capability = "make citations reproducible"
idempotency_key = "replan14:cli-cite"
labels = ["creates:crates/cadetta-cli/src/cmd/cite.rs", "creates:crates/cadetta-cli/src/cmd/mod.rs"]
scope = ["crates/cadetta-cli/src/cmd/cite.rs", "crates/cadetta-cli/src/lib.rs", "crates/cadetta-cli/Cargo.toml", "Cargo.lock", "docs/spec/config-cli.md", "crates/cadetta-cli/src/cmd/mod.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWSXBTNXW3CPZ6FVHT5ZT"

[[acceptance]]
text = "Given a URL citation, when cite pin NAME runs, then the file is stored in cites/ and the cite rewritten to the local path (config-cli.md 3)"
bound = true
+++
