+++
id = "01M4ZBN0XKQRTXW7AY1KKVE13B"
title = "Conformance import executor: accept/update modes and richer item features"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T08:42:53Z"
updated = "2026-10-15T08:42:53Z"
scope = ["crates/cadetta-cli/tests/**", "tests/conformance/known-gaps.toml"]
+++

Found while working ~JBHTKF3. The propose-mode import executor (tests/conformance import.rs) covers naming, ordering, driven skip. Remaining 22 import fixtures need: --accept/--update --write modelling (bindings, cad/imported, lock, refusal to overwrite, then_sync), property types Text/Count/Quantity and rename/needs_edit, equation_linked and $PRP skips, expression conversion, tolerance dropping, item_keys, assembly parts/instances, active-config info diagnostics, resolve-drift-import-update. Several need library work in cadetta-adapter import.
