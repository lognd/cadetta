+++
id = "01M4V1HED894WSWEEJ9K2NMWYA"
title = "Quantities: expose the display formatter (CLI/binding verb) for conformance"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-13T16:29:12Z"
updated = "2026-10-16T04:01:58Z"
idempotency_key = "Z48-display"
labels = ["creates:crates/cadetta-cli/src/cmd/display.rs", "creates:docs/schemas/display.json"]
scope = ["crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-cli/src/lib.rs", "crates/cadetta-cli/tests/**", "docs/spec/quantities.md", "docs/spec/config-cli.md", "crates/cadetta-cli/src/cmd/display.rs", "docs/schemas/display.json"]

[[acceptance]]
text = "Given a value and a display spec, when 'cadetta display VALUE SPEC' runs, then it prints the formatted text (JSON schema cadetta.display/1) and bad input exits 2"
bound = true
+++

found while working ~Z48GXKX. quantities.md 6; cadetta-units display exists but no surface. display-* fixtures are binding-call JSON {call,value,spec} with expected result; the runner needs to adopt that form.
