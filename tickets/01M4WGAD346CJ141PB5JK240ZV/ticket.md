+++
id = "01M4WGAD346CJ141PB5JK240ZV"
title = "Wire instances.rs default_impl to defaults::resolve_default"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "Claude"
created = "2026-10-14T06:06:44Z"
updated = "2026-10-14T11:49:31Z"
idempotency_key = "wire-default-impl"
scope = ["crates/cadetta-sema/src/instances.rs"]

[[acceptance]]
text = "Given component, file, project and std default layers, when default_impl resolves an interface, then it delegates to defaults::resolve_default and honors the precedence"
bound = true
+++
