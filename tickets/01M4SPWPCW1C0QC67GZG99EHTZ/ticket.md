+++
id = "01M4SPWPCW1C0QC67GZG99EHTZ"
title = "Name resolution and canonical paths"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWGB2729B40CQE81MHM2Z"
reporter = "lognd"
created = "2026-10-13T04:03:52Z"
updated = "2026-10-13T06:49:08Z"
persona = "design engineer"
capability = "reference parameters as `Other.param` or `part.param`"
outcome_text = "every value has one canonical path such as acme.rover::Axle.end_diameter"
labels = ["creates:crates/cadetta-sema/src/resolve.rs"]
scope = ["crates/cadetta-sema/src/resolve.rs", "crates/cadetta-sema/src/lib.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWPG1WBAHFPZ19ZV3BAPJ"

[[links]]
kind = "blocks"
target = "01M4SPWPKGXD3TJ78NPJH8Q9VZ"

[[acceptance]]
text = "Given a bare name in a component, when resolved, then it is the component's own parameter"
bound = true

[[acceptance]]
text = "Given declaration order reversed, when resolved, then results are identical"
bound = true

[[acceptance]]
text = "Given `Type.member` on a parameterized component whose member depends on no parameter, when resolved, then it reads without an instance; if it depends on a parameter an error names the parameter (language.md 3.1)"
bound = false

[[acceptance]]
text = "Given a parameterless component read by type name, when resolved, then it is one implicit instance shared by every read in the project (language.md 3.1)"
bound = false

[[acceptance]]
text = "Given a bare enum variant where the parameter type is known, when resolved, then it is accepted; elsewhere it must be qualified (language.md 3.1)"
bound = false

[[acceptance]]
text = "Given a canonical path with `/part[index]` segments, when resolved, then it matches ir.md 2"
bound = false

[[acceptance]]
text = "Given `Other.param`, when resolved, then the same module is searched before imported names (packages.md 4)"
bound = false

[[acceptance]]
text = "Given a canonical path package::module::Item.member, when resolved, then it matches ir.md 2 and packages.md 2"
bound = false
+++
