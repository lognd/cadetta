+++
id = "01M4V7WC7QPQ4RH2902T8PTE0A"
title = "sw-equations emit: R014 error for bound Text param that must be emitted"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T18:20:02Z"
updated = "2026-10-15T03:40:04Z"
scope = ["crates/cadetta-adapter/**", "crates/cadetta-cli/src/cmd/emit.rs", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/tests/r014_text_not_expressible.rs", "tests/conformance/known-gaps.toml", "changelog.d/01M4V7WC7QPQ4RH2902T8PTE0A.*"]

[[acceptance]]
text = "A bound Text parameter that the run must emit raises R014 (error, exit 1, file withheld), for global and dimension locators"
bound = true

[[acceptance]]
text = "A Text parameter filtered out by adapter paths or unbound is not emitted and not an error"
bound = true
+++

found while working ~WMAWH0R. Implement R014 per emitters/sw-equations.md; fixtures r014-* are skipped until the emit runner exists.
