+++
id = "01M51ZFFQP22HJSYHXXGQGAT9X"
title = "Emit singular OTS part members (ring.bore_nominal etc.) as per-instance IR members"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-16T09:07:52Z"
updated = "2026-10-16T09:38:42Z"
scope = ["crates/cadetta-sema/src/lower.rs", "crates/cadetta-sema/src/lower_array.rs"]
+++

found while working ~A1N7FAP: lower_instance_members emits only parts.len()==1 && members.len()==1 evaluation entries; golden 02 BearingUnit/ring.* and nested instance paths missing
