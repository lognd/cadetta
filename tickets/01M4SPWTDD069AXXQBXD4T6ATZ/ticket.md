+++
id = "01M4SPWTDD069AXXQBXD4T6ATZ"
title = "cadetta sync"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T04:03:56Z"
updated = "2026-10-14T14:38:36Z"
persona = "design engineer"
capability = "run the full compile, lint, push, regen, read back, drift loop"
outcome_text = "one command replaces open, import, rebuild, check"
labels = ["creates:crates/cadetta-cli/src/cmd/sync.rs", "creates:docs/schemas/sync.json", "creates:changelog.d/01M4SPWTDD069AXXQBXD4T6ATZ.added.md", "creates:tests/fixtures/sync"]
scope = ["crates/cadetta-cli/src/cmd/sync.rs", "crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-cli/src/lib.rs", "crates/cadetta-cli/src/cmd/emit.rs", "crates/cadetta-cli/src/cmd/import.rs", "docs/spec/config-cli.md", "docs/spec/adapters.md", "docs/schemas/sync.json", "changelog.d/01M4SPWTDD069AXXQBXD4T6ATZ.added.md", "crates/cadetta-adapter/src/session.rs", "tests/fixtures/sync"]

[[acceptance]]
text = "Given no changed digest and matching read-back, when sync runs, then nothing is written and no regen is triggered"
bound = true

[[acceptance]]
text = "Given an adapter without drive, when sync runs, then emit plus manual instructions are printed"
bound = true

[[acceptance]]
text = "Given sync flags and the line format push|skip|drift|converged|fail path locator old -> new, when run, then output and exit codes follow config-cli.md 3, 3.1"
bound = true
+++
