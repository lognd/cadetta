+++
id = "01M4V4GNV6SHXYSWMZ9WD0CWPQ"
title = "Spec: sw-equations ref-mode dimension with no bound global and the 'sign' on created globals"
type = "docs"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T17:21:13Z"
updated = "2026-10-13T17:21:13Z"
scope = ["docs/spec/emitters/sw-equations.md"]
+++

found while working ~KW6EQ0D: spec 1 says the emitter creates a mangled global for an unbound-name global but does not state that a ref-mode dimension lacking ref_to triggers that creation, nor that the created global takes native_unit from the dimension locator. Fixture global-created-name-no-dots assumes both. Also unstated: the dimension-line order key when one path has dimensions only. Decide and write the rules.
