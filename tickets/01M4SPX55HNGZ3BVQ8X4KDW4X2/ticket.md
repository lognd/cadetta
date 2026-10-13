+++
id = "01M4SPX55HNGZ3BVQ8X4KDW4X2"
title = "OQ-7: Adapter discovery and versioning"
type = "docs"
category = "triage"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T04:04:07Z"
updated = "2026-10-13T04:04:07Z"
labels = ["open-question"]
+++

Open question OQ-7 from the spec. Closes with an ADR in docs/decisions/.

Question: Adapter discovery and versioning (PATH lookup vs a lockfile of adapter versions).

Recommendation: `cadetta.toml` pins command and version; `initialize` must match.
