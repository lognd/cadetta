+++
id = "01M4V4GNV6SHXYSWMZ9WD0CWPQ"
title = "Spec: sw-equations ref-mode dimension with no bound global and the 'sign' on created globals"
type = "docs"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-13T17:21:13Z"
updated = "2026-10-13T17:59:38Z"
scope = ["docs/spec/emitters/sw-equations.md", "docs/spec/adapters.md", "tests/conformance/emitters/sw-equations/**"]

[[acceptance]]
text = "Given a ref-mode dimension with no ref_to, no bound global and no alias referent, when the spec is read, then adapters.md 3 and emitters/sw-equations.md 1 state that the emitter creates the path's mangled global, that its unit is the dimension locator native_unit (else the adapter declared native unit), and that it is written before the path's dimension lines"
bound = true

[[acceptance]]
text = "Given the conformance corpus, when run, then a pair covers the created-global unit (must-fire: native_unit in gives 1in; must-not-fire: no native_unit gives the adapter default)"
bound = true
+++

found while working ~KW6EQ0D: spec 1 says the emitter creates a mangled global for an unbound-name global but does not state that a ref-mode dimension lacking ref_to triggers that creation, nor that the created global takes native_unit from the dimension locator. Fixture global-created-name-no-dots assumes both. Also unstated: the dimension-line order key when one path has dimensions only. Decide and write the rules.
