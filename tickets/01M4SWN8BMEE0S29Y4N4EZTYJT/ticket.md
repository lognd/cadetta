+++
id = "01M4SWN8BMEE0S29Y4N4EZTYJT"
title = "Project model, root discovery and packages"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T05:44:40Z"
updated = "2026-10-13T06:38:27Z"
persona = "design engineer"
capability = "run from any subdirectory"
idempotency_key = "replan14:cli-project"
labels = ["creates:crates/cadetta-cli/src/project.rs"]
scope = ["crates/cadetta-cli/src/project.rs", "crates/cadetta-cli/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWSTRG1FFAJ0XM89ETBBN"

[[acceptance]]
text = "Given a package, when loaded, then its modules are the .cdt files under src sorted by path (so digests are deterministic), and a missing src, a src with no .cdt file, or a workspace member or path dependency without cadetta.toml is E005; a duplicate component name in one module is E003 (config-cli.md 1.3, 1.4; packages.md 2)"
bound = false

[[acceptance]]
text = "Given a command run inside a workspace member, when run, then it scopes to that member unless run at the workspace root, and discovery stops at the nearest cadetta.toml with [package] or [workspace] (config-cli.md 1.6)"
bound = false

[[acceptance]]
text = "Given --cwd or --config, when run, then root discovery walks up to the nearest cadetta.toml with [package] or [workspace] and exits 2 if none (config-cli.md 1.6)"
bound = false
+++
