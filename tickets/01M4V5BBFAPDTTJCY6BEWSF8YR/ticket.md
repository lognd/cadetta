+++
id = "01M4V5BBFAPDTTJCY6BEWSF8YR"
title = "Spec: json emitter dimension and ref-mode locators undefined"
type = "docs"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-13T17:35:47Z"
updated = "2026-10-13T18:06:12Z"
scope = ["docs/spec/emitters/json.md", "docs/spec/emitters/csv.md", "docs/spec/lints.md", "docs/spec/adapters.md", "docs/spec/config-cli.md", "docs/spec/README.md", "crates/cadetta-lint/src/engine.rs", "tests/conformance/emitters/json/**", "tests/conformance/emitters/csv/**"]

[[acceptance]]
text = "Given a bindings file with a dimension locator or mode=ref for the json adapter, when the spec is read, then emitters/json.md states whether emit skips, emits or rejects it and with which code"
bound = true

[[acceptance]]
text = "Given a dimension, property or frame locator, or a mode=ref binding, bound to a path the json or csv adapter must emit, when emit runs, then it is the error R015 naming the path and locator, the file is withheld and exit is 1"
bound = true

[[acceptance]]
text = "Given the conformance corpus, when run, then json and csv have R015 pairs including a not-fire where the same locator is filtered out or unbound"
bound = true
+++

Found while working ~5RCYHAJ. emitters/json.md 1 says name is the bound locator (kind global) or the mangled path, and 3 says ref mode does not exist. It does not say what a dimension-kind locator, or a mode=ref binding, does on emit (skipped? emitted as a param named by its locator? configuration error?). No fixture written; the answer decides the diagnostic.
