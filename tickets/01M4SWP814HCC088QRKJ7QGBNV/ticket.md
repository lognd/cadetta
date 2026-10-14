+++
id = "01M4SWP814HCC088QRKJ7QGBNV"
title = "ScrewJoint one joint per pattern"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:45:12Z"
updated = "2026-10-14T12:49:25Z"
persona = "design engineer"
capability = "model a bolt pattern once"
idempotency_key = "replan14:sd-screw"
labels = ["creates:std/src/fasteners.cdt"]
scope = ["std/src/fasteners.cdt", "crates/cadetta-sema/src/instances.rs", "docs/spec/stdlib.md", "tests/conformance/known-gaps.toml"]

[[links]]
kind = "blocked-by"
target = "01M4STAPCEF5EPEYX0MZDSBA7J"

[[links]]
kind = "blocked-by"
target = "01M4STAPFPQ9GV8XJB4XCK8EGP"

[[links]]
kind = "blocked-by"
target = "01M4SWG6XQ2M0GR620Q6MJE3XF"

[[acceptance]]
text = "Given a ScrewJoint over a pattern, when instantiated, then exactly one joint covers the whole pattern; count versus count(at) is distinguished (stdlib.md 3)"
bound = false

[[acceptance]]
text = "Given part screws[4] used as a joint, when compiled, then E006 fires (stdlib.md 3; lints.md 3 E)"
bound = false
+++
