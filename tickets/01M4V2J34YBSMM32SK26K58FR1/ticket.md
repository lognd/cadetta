+++
id = "01M4V2J34YBSMM32SK26K58FR1"
title = "std commercial, fits, materials and module diagnostics not raised (I001 Q003 F001 E002)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:47:02Z"
updated = "2026-10-15T03:26:41Z"
scope = ["tests/conformance/stdlib/**", "crates/cadetta-sema/src/kindcheck.rs", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/tests/stdlib_interface_kinds.rs", "docs/spec/stdlib.md", "tests/conformance/known-gaps.toml", "changelog.d/**"]

[[acceptance]]
text = "A wrong dimension on an implementer interface member is I001 (stdlib/i001-sourced-wrong-kind fires, must-not-fire is clean)"
bound = true

[[acceptance]]
text = "A wrong named kind on an implementer interface member is Q003 (stdlib/q003-sourced-named-kind)"
bound = true

[[acceptance]]
text = "Optional Sourced members may be absent without I001 (stdlib/i001-sourced-optional-members)"
bound = false
+++

Fixtures failing against the binary, gapped in known-gaps.toml: commercial-misspelling-not-a-member e002-module-not-shipped f001-j-outside-it5-7 i001-sourced-optional-members i001-sourced-wrong-kind materials-density-units materials-tap-class q003-sourced-named-kind screw-price-not-in-iso4762. Found while working ~GDYVJ8R.
