+++
id = "01M4V4GNQC1CRVZCB9GWMAWH0R"
title = "Spec: sw-equations behaviour when a Text parameter is bound"
type = "docs"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-13T17:21:12Z"
updated = "2026-10-13T17:56:24Z"
scope = ["docs/spec/emitters/sw-equations.md", "docs/spec/lints.md", "docs/spec/adapters.md", "docs/spec/config-cli.md", "docs/spec/README.md", "crates/cadetta-lint/src/engine.rs", "tests/conformance/emitters/sw-equations/**"]

[[acceptance]]
text = "Given a bound Text parameter that an sw-equations emit must write, when the spec is read, then emitters/sw-equations.md, adapters.md and lints.md state that it is the error R014 (withholds the file, exit 1) and that a Text parameter never emitted (out of scope, filtered, or unbound) is not an error"
bound = true

[[acceptance]]
text = "Given the conformance corpus, when run, then value-text-not-emitted and a new R014 pair encode the rule (must-fire: error R014; must-not-fire: Text param present but not bound or filtered out)"
bound = false
+++

found while working ~KW6EQ0D: emitters/sw-equations.md 1 says Text is not emitted but not whether a bound Text path is silently skipped, warned, or a configuration error. Fixture value-text-not-emitted assumes silent skip with no diagnostic. Decide and write the rule.
