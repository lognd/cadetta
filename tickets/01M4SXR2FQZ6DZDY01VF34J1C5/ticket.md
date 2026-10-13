+++
id = "01M4SXR2FQZ6DZDY01VF34J1C5"
title = "Dependency resolution from path and git, one version per package"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 8
parent = "01M4SXQPADRSNTEY3J7PH899Y8"
reporter = "lognd"
created = "2026-10-13T06:03:40Z"
updated = "2026-10-13T07:00:02Z"
persona = "product engineer"
capability = "depend on a shop library by path or git and get one resolved version"
labels = ["creates:crates/cadetta-cli/src/pkg/resolve.rs"]
scope = ["crates/cadetta-cli/src/pkg/resolve.rs", "docs/spec/packages.md", "crates/cadetta-cli/src/pkg/mod.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SXQWGAYJDGMGKNBMHASNBA"

[[acceptance]]
text = "Given path and git (tag, rev, branch) dependencies, when resolved, then each yields a package directory with a cadetta.toml, and a missing one is E005 (packages.md 1; config-cli.md 1.3)"
bound = true

[[acceptance]]
text = "Given a resolved package whose version fails the `version` semver requirement, when resolved, then E012 fires (packages.md 1, 7)"
bound = true

[[acceptance]]
text = "Given two requirements on one package that no single version meets, when resolved, then E013 fires and no second copy is loaded (packages.md 1, 7)"
bound = true

[[acceptance]]
text = "Given transitive dependencies, when resolved, then the whole project has exactly one version of each package, deterministically ordered (packages.md 1)"
bound = true
+++
