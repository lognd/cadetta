+++
id = "01M4SWN8BMEE0S29Y4N4EZTYJT"
title = "Project model, root discovery and namespaces"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T05:44:40Z"
updated = "2026-10-13T05:44:44Z"
persona = "design engineer"
capability = "run from any subdirectory"
idempotency_key = "replan14:cli-project"
labels = ["creates:crates/cadetta-cli/src/project.rs"]
scope = ["crates/cadetta-cli/src/project.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWSTRG1FFAJ0XM89ETBBN"

[[acceptance]]
text = "Given --cwd or --config, when run, then root discovery walks up to cadetta.toml and exits 2 if none (config-cli.md 1.7)"
bound = false

[[acceptance]]
text = "Given sources globs, when expanded, then matches are sorted, may use .., and a glob matching nothing is E005; a duplicate component name is E003 (config-cli.md 1)"
bound = false
+++
