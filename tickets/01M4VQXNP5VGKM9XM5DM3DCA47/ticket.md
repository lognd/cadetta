+++
id = "01M4VQXNP5VGKM9XM5DM3DCA47"
title = "Spec: module path notation differs between adapters.md 3.2 (dotted) and packages.md 2 (file-tree, E015)"
type = "docs"
category = "todo"
priority = "medium"
points = 2
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:00:21Z"
updated = "2026-10-13T23:22:08Z"
scope = ["docs/spec/**", "tests/conformance/adapters/scope-*", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "Spec and fixtures agree; adapters/scope-* fixtures pass."
bound = false
+++

found while working ~6BCVN48: adapters/scope-* fixtures use src/gear.sub.cdt, which packages.md flags as E015. Owner decision needed on the module path form in bindings scope.modules/paths; then fix spec or fixtures.
