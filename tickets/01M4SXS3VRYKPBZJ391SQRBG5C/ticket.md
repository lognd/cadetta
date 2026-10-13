+++
id = "01M4SXS3VRYKPBZJ391SQRBG5C"
title = "Upstream attestations and trust roles"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SXQPADRSNTEY3J7PH899Y8"
reporter = "lognd"
created = "2026-10-13T06:04:15Z"
updated = "2026-10-13T06:04:15Z"
persona = "product engineer"
capability = "accept a library sign-off I did not make"
labels = ["creates:crates/cadetta-obligations/src/trust.rs"]
scope = ["crates/cadetta-obligations/src/trust.rs"]

[[links]]
kind = "blocked-by"
target = "01M4ST8F3SDF95MVP07VXXNX0X"

[[links]]
kind = "blocked-by"
target = "01M4SXQWGAYJDGMGKNBMHASNBA"

[[links]]
kind = "blocked-by"
target = "01M4SXR720K3VGM1K7DWZ0VJ7Z"

[[acceptance]]
text = 'Given a dependency lock carrying attestations of its own nodes, when loaded, then they travel with the package and `cadetta obligations` shows them as "attested upstream" (packages.md 6)'
bound = false

[[acceptance]]
text = 'Given `[dependencies."acme.shop"] trust = ["source", "tolerance_review"]`, when judged, then those roles discharge the dependent matching obligations on the dependency nodes (packages.md 6)'
bound = false

[[acceptance]]
text = "Given no trust key, when judged, then upstream attestations are information only and discharge nothing (packages.md 6)"
bound = false
+++
