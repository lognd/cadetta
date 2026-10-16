+++
id = "01M4YKTX2YMSXVE4CK0JAVMC41"
title = "IR: define a Require entity shape and Project requires[]"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T01:46:40Z"
updated = "2026-10-16T07:47:36Z"
scope = ["crates/cadetta-ir/**", "docs/spec/ir.md", "bindings/python/python/cadetta/models/ir.py", "docs/schemas/cadetta.ir.1.json"]

[[acceptance]]
text = "Require entity serializes, decodes, digests and validates against cadetta.ir.1.json; Project.requires[] optional"
bound = true
+++

found while working ~XSHCDWP: ir.md 3 lists requires[] but no Require entity shape exists; needs a design decision before the model/schema can carry it.
