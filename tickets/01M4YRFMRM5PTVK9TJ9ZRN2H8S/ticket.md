+++
id = "01M4YRFMRM5PTVK9TJ9ZRN2H8S"
title = "registry dependency and reserved std name should be E005 diagnostics"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T03:07:53Z"
updated = "2026-10-16T10:07:38Z"
scope = ["crates/cadetta-cli/**", "changelog.d/**", "tests/conformance/packages/**", "docs/spec/packages.md", "docs/spec/config-cli.md", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "A registry-only dependency and a package, member or dependency named std are reported as E005, not a config error"
bound = false
+++

found while working ~AN7BAC5. Fixtures packages/e005-registry-dependency-rejected and packages/e005-std-name-reserved expect E005 but check exits 2 with a config parse/invalid error. packages.md only says 'rejected as unsupported registry source' and does not name E005 for either, so this needs a spec decision (E005 vs config error) before implementing.
