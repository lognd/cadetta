+++
id = "01M4VVBYRVM2SEPC3F1HX1YH7M"
title = "One module-prefix matcher: CLI --module mishandles dotted package prefixes"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-14T00:00:35Z"
updated = "2026-10-14T02:14:34Z"
scope = ["crates/cadetta-cli/src/scope.rs", "crates/cadetta-adapter/src/scope.rs"]

[[acceptance]]
text = 'Given --module acme.gear and [[adapter]] modules = ["acme.gear"], when scoping, then both match packages acme.gear and acme.gear.sub but not acme.gearbox through one shared matcher in cadetta-adapter that the CLI reuses (config-cli.md 3; adapters.md 3.2)'
bound = false
+++

Found by ~M3DCA47: ModulePrefix::matches in the CLI compares dotted package segments by first dot-part, so acme.gear misses acme.gear.sub; the adapter has its own correct matcher.
