+++
id = "01M558W4P4ZETDB9F923651CEJ"
title = "Read-back mismatch when CAD has a tolerance the design lacks"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T15:49:47Z"
updated = "2026-10-17T16:17:39Z"
scope = ["crates/cadetta-adapter/**"]

[[acceptance]]
text = "Given CAD has a tolerance the design lacks, when read-back is matched, then the locator matches"
bound = true
+++

Found while working ~NEHY9X7 (nova live smoke). matched_locators requires tolerance equality, so sync reports a mismatch.
