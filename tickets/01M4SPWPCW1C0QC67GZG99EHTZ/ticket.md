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
updated = "2026-10-13T05:01:23Z"
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

[[acceptance]]
text = "Given `use std.fits::iso286` and `use a.b::{X, Y as Z}`, when resolved, then imports bring names into scope (language.md 3, 4)"
bound = false

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
+++
