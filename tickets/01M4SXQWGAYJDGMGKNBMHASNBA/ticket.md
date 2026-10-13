+++
id = "01M4SXQWGAYJDGMGKNBMHASNBA"
title = "Package manifest and workspace model"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SXQPADRSNTEY3J7PH899Y8"
reporter = "lognd"
created = "2026-10-13T06:03:34Z"
updated = "2026-10-13T06:36:42Z"
persona = "library author"
capability = "declare a package with dependencies, or group packages in a workspace"
labels = ["creates:crates/cadetta-cli/src/pkg/manifest.rs"]
scope = ["crates/cadetta-cli/src/pkg/manifest.rs", "crates/cadetta-cli/src/pkg/mod.rs", "docs/spec/packages.md"]

[[links]]
kind = "blocked-by"
target = "01M4SPWSTRG1FFAJ0XM89ETBBN"

[[acceptance]]
text = 'Given a cadetta.toml with [package] name, version and src, when loaded, then name must be lower.dotted (N001), version is semver, src defaults to "src" and the package root is src/lib.cdt (packages.md 1, 2; config-cli.md 2)'
bound = true

[[acceptance]]
text = "Given [dependencies] entries with path, or git plus tag, rev or branch, when loaded, then each is typed; a registry source is rejected as unsupported (packages.md 1; OQ-19)"
bound = true

[[acceptance]]
text = "Given a package named std, when loaded, then it is rejected, and std is an implicit dependency at the compiler version (packages.md 1)"
bound = true

[[acceptance]]
text = "Given a cadetta.toml with [workspace] members, when loaded, then members share one lock and one build/ and may depend on each other by path; a member without cadetta.toml is E005 (packages.md 1; config-cli.md 1.3)"
bound = true

[[acceptance]]
text = 'Given [dependencies."acme.shop"] trust = ["source"], when loaded, then trust roles are parsed (packages.md 6)'
bound = false
+++
