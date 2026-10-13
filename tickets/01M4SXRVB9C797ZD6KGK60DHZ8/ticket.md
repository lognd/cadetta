+++
id = "01M4SXRVB9C797ZD6KGK60DHZ8"
title = "Lint U002 unused private item and I004 scope"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SXQPADRSNTEY3J7PH899Y8"
reporter = "lognd"
created = "2026-10-13T06:04:06Z"
updated = "2026-10-13T06:04:28Z"
persona = "library author"
capability = "find dead private items"
labels = ["creates:crates/cadetta-lint/src/rules/u002.rs"]
scope = ["crates/cadetta-lint/src/rules/u002.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWQM289M7Q58CSS6F029F"

[[links]]
kind = "blocked-by"
target = "01M4SXRFXB1JYNPNKSAY6WB3QC"

[[links]]
kind = "blocked-by"
target = "01M4SXRV8HCNXHVFX87BTFJC7S"

[[acceptance]]
text = "Given a private item used nowhere in its package, when linted, then U002 warning fires; used once it does not (packages.md 3, 7; lints.md U)"
bound = false

[[acceptance]]
text = "Given a private parameterized component with no instances, when linted, then U002 fires and I004 does not; I004 fires only for roots of the package being checked (packages.md 3; lints.md I004)"
bound = false

[[acceptance]]
text = "Given U002 and E012-E017, when shipped, then lints.md catalog entries, severities (fixed errors for E012-E017) and must-fire and must-not-fire fixtures exist under tests/conformance/lints/<code>/ (lints.md 3, 4)"
bound = false
+++
