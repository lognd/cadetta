+++
id = "01M4YRFMRM5PTVK9TJ9ZRN2H8S"
title = "registry dependency and reserved std name should be E005 diagnostics"
type = "bug"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T03:07:53Z"
updated = "2026-10-16T09:23:46Z"
scope = ["crates/cadetta-cli/**", "changelog.d/**", "docs/**", "tests/conformance/packages/**"]
+++

found while working ~AN7BAC5. Fixtures packages/e005-registry-dependency-rejected and packages/e005-std-name-reserved expect E005 but check exits 2 with a config parse/invalid error. packages.md only says 'rejected as unsupported registry source' and does not name E005 for either, so this needs a spec decision (E005 vs config error) before implementing.
