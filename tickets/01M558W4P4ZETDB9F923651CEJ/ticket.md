+++
id = "01M558W4P4ZETDB9F923651CEJ"
title = "Read-back mismatch when CAD has a tolerance the design lacks"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-17T15:49:47Z"
updated = "2026-10-17T15:49:47Z"
scope = ["crates/cadetta-adapter/**"]
+++

Found while working ~NEHY9X7 (nova live smoke). matched_locators requires tolerance equality, so sync reports a mismatch.
