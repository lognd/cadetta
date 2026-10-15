+++
id = "01M4V7WDC104ENF4T0V0HEFFN7"
title = "sw-equations emit: created global unit from locator native_unit else adapter default"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T18:20:03Z"
updated = "2026-10-15T04:07:47Z"
scope = ["crates/cadetta-adapter/**", "crates/cadetta-cli/src/cmd/emit.rs", "tests/conformance/emitters/sw-equations/**", "tests/conformance/known-gaps.toml", "changelog.d/**"]

[[acceptance]]
text = "The created global takes the ref-mode dimension locator native_unit, else the adapter native unit"
bound = false
+++

found while working ~WD0CWPQ. See adapters.md 3 and sw-equations.md 1.
