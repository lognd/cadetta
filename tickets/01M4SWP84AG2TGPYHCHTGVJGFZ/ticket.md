+++
id = "01M4SWP84AG2TGPYHCHTGVJGFZ"
title = "[defaults] precedence"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4STANRAWHV180C7RH0NQAY9"
reporter = "lognd"
created = "2026-10-13T05:45:12Z"
updated = "2026-10-14T05:56:18Z"
persona = "design engineer"
capability = "override policies per project"
idempotency_key = "replan14:sd-defaults"
labels = ["creates:crates/cadetta-sema/src/defaults.rs"]
scope = ["crates/cadetta-sema/src/defaults.rs", "crates/cadetta-sema/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWSTRG1FFAJ0XM89ETBBN"

[[links]]
kind = "blocked-by"
target = "01M4STAP93AYTZDJYWCCAS1TRH"

[[acceptance]]
text = "Given #![default] on component and file, project [defaults] and std, when resolved, then precedence is that order, highest first (stdlib.md 4; config-cli.md 2)"
bound = true
+++
