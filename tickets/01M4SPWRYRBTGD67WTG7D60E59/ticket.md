+++
id = "01M4SPWRYRBTGD67WTG7D60E59"
title = "Bindings files, multi-locator bindings and name mangling"
type = "story"
category = "todo"
priority = "medium"
points = 8
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T04:03:54Z"
updated = "2026-10-13T05:44:12Z"
persona = "design engineer"
capability = "get bindings/<adapter>.toml and deterministic default names"
outcome_text = "most bindings never need to be written"
labels = ["creates:crates/cadetta-adapter/src/bindings.rs"]
scope = ["crates/cadetta-adapter/src/bindings.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWSABPXKK62B5V0SMJ8FD"

[[acceptance]]
text = "Given `bindings/<adapter>.toml` with one table per canonical path and several locators, when loaded, then each locator keeps kind (global|dimension|property), mode and native_unit (adapters.md 3)"
bound = false

[[acceptance]]
text = "Given a dimension locator with `ref_to`, when resolved, then it points at that global; else at the first global locator of the same path in file order; else at the global of its referent if a pure alias; else at the path own mangled global (adapters.md 3)"
bound = false

[[acceptance]]
text = "Given `Axle.end_diameter` and a target forbidding dots, when mangled, then the name is `Axle__end_diameter`; array instances mangle to `Rover__wheel_2__hub_bore`; mangling is injective (adapters.md 3)"
bound = false

[[acceptance]]
text = "Given a parameter with `was`, when renamed, then its binding carries forward (adapters.md 3)"
bound = false

[[acceptance]]
text = 'Given `#[native(adapter, "locator")]` in source, when loaded, then it overrides the file entry (adapters.md 3)'
bound = false

[[acceptance]]
text = "Given an unbound parameter or a `let`, when emitting, then it is not written (adapters.md 3)"
bound = false

[[acceptance]]
text = "Given a binding whose path matches nothing, when loaded, then it is reported for R005 (adapters.md 3)"
bound = false

[[acceptance]]
text = "Given bindings, when loaded, then multi-locator, mangling and all ir.md 3 Locator fields parse (adapters.md 3)"
bound = false
+++
