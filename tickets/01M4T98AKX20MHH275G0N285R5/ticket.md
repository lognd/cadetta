+++
id = "01M4T98AKX20MHH275G0N285R5"
title = "Conformance fixtures for U002, I004 and E012-E017"
type = "story"
category = "todo"
priority = "medium"
parent = "01M4SXQPADRSNTEY3J7PH899Y8"
reporter = "lognd"
created = "2026-10-13T09:24:47Z"
updated = "2026-10-13T09:24:47Z"
labels = ["creates:tests/conformance/lints/**"]
scope = ["tests/conformance/lints/**"]

[[links]]
kind = "blocked-by"
target = "01M4SXRVB9C797ZD6KGK60DHZ8"

[[acceptance]]
text = "Given U002, I004 and E012-E017, when shipped, then must-fire and must-not-fire fixtures exist under tests/conformance/lints/<code>/ with severities per lints.md catalog (fixed errors for E012-E017) (lints.md 3, 4)"
bound = false
+++

Split from ~K60DHZ8 criterion 3. Held pending owner sign-off on the expect.json format in tests/conformance/README.md. Do not start before that.
