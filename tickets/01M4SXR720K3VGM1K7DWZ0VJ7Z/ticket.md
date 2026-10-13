+++
id = "01M4SXR720K3VGM1K7DWZ0VJ7Z"
title = "Lock package records with content digests and E014"
type = "story"
category = "done"
outcome = "done"
priority = "medium"
points = 5
parent = "01M4SXQPADRSNTEY3J7PH899Y8"
reporter = "lognd"
created = "2026-10-13T06:03:45Z"
updated = "2026-10-13T07:58:07Z"
persona = "library user"
capability = "know when a pinned dependency changed"
labels = ["creates:crates/cadetta-obligations/src/lock_package.rs"]
scope = ["crates/cadetta-obligations/src/lock_package.rs", "crates/cadetta-obligations/src/lib.rs", "docs/spec/packages.md"]

[[links]]
kind = "blocked-by"
target = "01M4SPWQED1EN19NWQJE5NT0J4"

[[links]]
kind = "blocked-by"
target = "01M4ST8EPKBWRWHF51Y5NV3VK0"

[[links]]
kind = "blocked-by"
target = "01M4SXQWGAYJDGMGKNBMHASNBA"

[[acceptance]]
text = "Given a resolved dependency, when the lock is written, then a line `package NAME version=V source=S digest=HEX` is stored, keyed by NAME and sorted with the other records (obligations.md 3; packages.md 1)"
bound = true

[[acceptance]]
text = "Given a dependency whose content digest differs from its pinned record, when checked, then E014 fires and names `cadetta update` (packages.md 1, 7)"
bound = true

[[acceptance]]
text = "Given the same dependency content twice, when digested, then the digest is identical regardless of checkout path (packages.md 1; ir.md 4)"
bound = true

[[acceptance]]
text = "Given a dependency digest, when the closure digest of a reader is computed, then the digest is an input to it (obligations.md 3; packages.md 6)"
bound = true
+++
