+++
id = "01M4W3YJCH948JJQ3HY646D1T7"
title = "sema: evaluate parameterized component instances (std ScrewJoint members)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 13
reporter = "lognd"
created = "2026-10-14T02:30:34Z"
updated = "2026-10-14T02:33:57Z"
scope = ["crates/cadetta-sema/**", "std/**", "tests/**"]

[[acceptance]]
text = "Given a design instantiating std::fasteners ScrewJoint with M5 medium, when cadetta check runs, then its members evaluate to the ISO table values with no E002/EVAL errors"
bound = false
+++

found while working ~4BJ1ZGM: the evaluator is path-keyed and skips parameterized components (index_component); needs arg binding and defaults, optional args/given(), #[when], count(at), part arrays, policy-default resolution and fmt (known-gaps stdlib/joint-* and policy-* entries).
