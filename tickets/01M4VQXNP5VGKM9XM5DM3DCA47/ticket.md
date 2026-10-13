+++
id = "01M4VQXNP5VGKM9XM5DM3DCA47"
title = "Spec: module path notation differs between adapters.md 3.2 (dotted) and packages.md 2 (file-tree, E015)"
type = "docs"
category = "in-progress"
priority = "medium"
points = 2
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:00:21Z"
updated = "2026-10-13T23:54:56Z"
scope = ["tests/conformance/adapters/scope-*", "tests/conformance/known-gaps.toml", "docs/spec/adapters.md", "docs/spec/config-cli.md", "crates/cadetta-adapter/src/scope.rs"]

[[acceptance]]
text = "Spec and fixtures agree; adapters/scope-* fixtures pass."
bound = true
+++

found while working ~6BCVN48: adapters/scope-* fixtures use src/gear.sub.cdt, which packages.md flags as E015. Owner decision needed on the module path form in bindings scope.modules/paths; then fix spec or fixtures.
