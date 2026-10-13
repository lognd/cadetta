+++
id = "01M4SXS3YDG59YAC0QXAMP4S5F"
title = "Conformance run for packages"
type = "story"
category = "todo"
priority = "medium"
points = 3
parent = "01M4SXQPADRSNTEY3J7PH899Y8"
reporter = "lognd"
created = "2026-10-13T06:04:15Z"
updated = "2026-10-13T06:04:15Z"
persona = "maintainer"
capability = "run the packages.md fixtures in CI"
labels = ["creates:tests/conformance_packages.rs"]
scope = ["tests/conformance_packages.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SXRFQ0RENW7YM154W0EWP9"

[[links]]
kind = "blocked-by"
target = "01M4SXRV8HCNXHVFX87BTFJC7S"

[[links]]
kind = "blocked-by"
target = "01M4SXRVB9C797ZD6KGK60DHZ8"

[[acceptance]]
text = "Given tests/conformance/packages/ fixtures, when the harness runs, then every must-fire fixture produces its expected diagnostic and every must-not-fire fixture produces none (docs/spec/packages.md)"
bound = false
+++
