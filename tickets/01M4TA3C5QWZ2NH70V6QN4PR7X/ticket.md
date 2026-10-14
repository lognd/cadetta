+++
id = "01M4TA3C5QWZ2NH70V6QN4PR7X"
title = "obligations lists ots and citation obligations"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T09:39:34Z"
updated = "2026-10-14T23:05:53Z"
scope = ["crates/cadetta-obligations", "crates/cadetta-cli/src/universe.rs", "docs/spec/config-cli.md", "changelog.d/*", "tests/conformance/obligations/s004-citation-changed.must-fire/*", "crates/cadetta-cli/src/cmd/check.rs"]

[[acceptance]]
text = "Given an ots component citing a file, when 'cadetta obligations' runs with no attestation, then ots:PATH and citation:PATH are listed open"
bound = true

[[acceptance]]
text = "Given the ots acknowledged, when the cited file changes, then citation:PATH is listed stale and ots:PATH is not"
bound = true
+++

found while working ~E631Y52. requires, fits, citations, ots, sync, waiver, S003/S004/O004 not enumerated; check closures lack read-member deps.
