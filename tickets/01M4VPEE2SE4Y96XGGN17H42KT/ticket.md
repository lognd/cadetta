+++
id = "01M4VPEE2SE4Y96XGGN17H42KT"
title = "bind --tolerance drive|ignore writes the locator tolerance field"
type = "task"
category = "in-progress"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T22:34:33Z"
updated = "2026-10-13T23:13:57Z"
scope = ["crates/cadetta-cli/src/cmd/bind.rs", "crates/cadetta-adapter/src/bindings.rs", "docs/spec/config-cli.md"]

[[links]]
kind = "blocked-by"
target = "01M4VB648PMGRPG6R8D606HZK6"

[[acceptance]]
text = "Given cadetta bind --tolerance drive|ignore on a dimension locator, when run, then the locator tolerance field is written canonically and repeating it changes nothing (config-cli.md 3)"
bound = false
+++

found while working ~4T5YM3Y: bind exits 2 on --tolerance until bindings carry a tolerance field (~606HZK6 adds it to the model).
