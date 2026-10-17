+++
id = "01M4ZAD0AW4C1R33F43WTVTR3V"
title = "Part arrays of parameterized callees do not lower instance members"
type = "task"
category = "done"
outcome = "done"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T08:21:01Z"
updated = "2026-10-15T23:41:07Z"
scope = ["crates/cadetta-sema/src/**", "docs/spec/ir.md"]

[[acceptance]]
text = "A counted part of a parameterized callee lowers one instance per index with per-instance values"
bound = true
+++

Found while working ~9WDH9XE. lower_array.rs handles only parameter-less callees (all instances share the callee's definitions). A counted part with arguments (instances.rs::materialise skips it) has per-instance values and no IR members. Independent per-instance tolerance variables are not modelled either.
