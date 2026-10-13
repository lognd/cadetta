+++
id = "01M4SXS3RXGHEG5FGTCS7X071X"
title = "Dependency digests in closures and upgrade verdict flips"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SXQPADRSNTEY3J7PH899Y8"
reporter = "lognd"
created = "2026-10-13T06:04:14Z"
updated = "2026-10-13T09:08:57Z"
persona = "product engineer"
capability = "see exactly which checks a library update makes stale"
labels = ["creates:crates/cadetta-obligations/src/pkg_closure.rs"]
scope = ["crates/cadetta-obligations/src/pkg_closure.rs", "crates/cadetta-obligations/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4ST8ERHSJXGDXNH75ZCX6H7"

[[links]]
kind = "blocked-by"
target = "01M4SXR720K3VGM1K7DWZ0VJ7Z"

[[acceptance]]
text = "Given a dependency content digest, when closure digests are computed, then it is part of the closure of everything that reads the dependency (packages.md 6; obligations.md 3)"
bound = true

[[acceptance]]
text = "Given a library update, when checked, then exactly the dependents checks, sign-offs and CAD syncs that read changed values are stale and others stay discharged (packages.md 6)"
bound = true

[[acceptance]]
text = "Given a library update that flips a dependent verdict, when `cadetta check` runs, then the flip is reported through the obligation graph whatever the version number says (packages.md 5)"
bound = true
+++
