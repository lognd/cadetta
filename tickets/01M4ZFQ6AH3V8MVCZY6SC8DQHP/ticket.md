+++
id = "01M4ZFQ6AH3V8MVCZY6SC8DQHP"
title = "Lower ScrewJoint floats for joints inside sub-assembly components"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T09:53:58Z"
updated = "2026-10-15T09:53:58Z"
scope = ["crates/cadetta-sema/src/boltfloat.rs"]
+++

found while working ~4XTSTXR: bolt_floats only walks the top-level assembly; joints in sub-assembly components add no float.
