+++
id = "01M4V5BBFAPDTTJCY6BEWSF8YR"
title = "Spec: json emitter dimension and ref-mode locators undefined"
type = "docs"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T17:35:47Z"
updated = "2026-10-13T17:35:47Z"
scope = ["docs/spec/emitters/json.md"]

[[acceptance]]
text = "Given a bindings file with a dimension locator or mode=ref for the json adapter, when the spec is read, then emitters/json.md states whether emit skips, emits or rejects it and with which code"
bound = false
+++

Found while working ~5RCYHAJ. emitters/json.md 1 says name is the bound locator (kind global) or the mangled path, and 3 says ref mode does not exist. It does not say what a dimension-kind locator, or a mode=ref binding, does on emit (skipped? emitted as a param named by its locator? configuration error?). No fixture written; the answer decides the diagnostic.
