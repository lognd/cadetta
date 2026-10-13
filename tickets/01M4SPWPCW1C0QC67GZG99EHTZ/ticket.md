+++
id = "01M4SPWPCW1C0QC67GZG99EHTZ"
title = "Name resolution and canonical paths"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T04:03:58Z"
persona = "design engineer"
capability = "reference parameters as `Other.param` or `part.param`"
outcome_text = "every value has one canonical path such as acme.rover::Axle.end_diameter"
labels = ["creates:crates/cadetta-sema/src/resolve.rs"]
scope = ["crates/cadetta-sema/src/resolve.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWPG1WBAHFPZ19ZV3BAPJ"

[[links]]
kind = "blocks"
target = "01M4SPWPKGXD3TJ78NPJH8Q9VZ"

[[acceptance]]
text = "Given a bare name in a component, when resolved, then it is the component's own parameter"
bound = false

[[acceptance]]
text = "Given `Other.param`, when resolved, then the same namespace is searched before imported namespaces"
bound = false

[[acceptance]]
text = "Given declaration order reversed, when resolved, then results are identical"
bound = false
+++
