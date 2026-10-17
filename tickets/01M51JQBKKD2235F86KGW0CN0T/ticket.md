+++
id = "01M51JQBKKD2235F86KGW0CN0T"
title = "digest add_fit should skip accessor nodes already emitted as members"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-16T05:24:58Z"
updated = "2026-10-16T09:11:03Z"
scope = ["crates/cadetta-ir/src/digest.rs"]

[[acceptance]]
text = "add_fit keeps <fit>.hole/.shaft nodes already emitted as members"
bound = true
+++

found while working ~93GZBM2: add_fit overwrites <fit>.hole/.shaft member nodes now that sema emits them; deterministic but should skip existing.
