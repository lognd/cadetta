+++
id = "01M4V7WCM9TBRWY007WWS81S98"
title = "json/csv emit: R015 error for dimension/property/frame locator or mode=ref"
type = "task"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T18:20:02Z"
updated = "2026-10-15T03:43:04Z"
scope = ["crates/cadetta-adapter/**", "crates/cadetta-cli/**", "changelog.d/**", "tests/conformance/known-gaps.toml", "docs/**"]

[[acceptance]]
text = "Given a dimension, property or frame locator, or a mode=ref binding, bound to a path the json or csv adapter must emit, when emit runs, then it is the error R015 naming the path, the file is withheld and exit is 1"
bound = true

[[acceptance]]
text = "Given such a locator that is filtered out or unbound, when emit runs, then R015 is not raised"
bound = true
+++

found while working ~EWSF8YR. Implement R015 per json.md 3 and csv.md.
