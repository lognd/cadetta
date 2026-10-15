+++
id = "01M4SWG70T71SNTZ2TWXZD1Y50"
title = "Lint G005 bolted-float-unlocated"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SWEQMQ8DQMN8B8GWVR4GDH"
reporter = "lognd"
created = "2026-10-13T05:41:54Z"
updated = "2026-10-15T10:07:58Z"
persona = "design engineer"
capability = "see when a tight check ignores bolt float"
idempotency_key = "replan14:geo-g005"
labels = ["creates:crates/cadetta-lint/src/rules/g005.rs", "creates:tests/conformance/lints/G005/**"]
scope = ["crates/cadetta-lint/src/rules/g005.rs", "tests/conformance/lints/G005/**", "crates/cadetta-sema/src/checks.rs", "crates/cadetta-sema/tests/bolt_float.rs", "docs/spec/geometry.md", "tests/conformance/known-gaps.toml"]

[[links]]
kind = "blocked-by"
target = "01M4SPWQM289M7Q58CSS6F029F"

[[links]]
kind = "blocked-by"
target = "01M4Y2Z95B99FTVX68T4XTSTXR"

[[acceptance]]
text = "Given a check tighter than a joint's float that reads through a placement carrying it, when linted, then G005 warning fires (geometry.md 5.2, 7)"
bound = false

[[acceptance]]
text = "Given locating = true or a looser check, when linted, then G005 does not fire (geometry.md 5.2)"
bound = false
+++
