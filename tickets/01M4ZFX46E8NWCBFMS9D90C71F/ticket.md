+++
id = "01M4ZFX46E8NWCBFMS9D90C71F"
title = "docs: adapters.md 2.3 still says Backend::push exists; sync is Document-only"
type = "docs"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-15T09:57:13Z"
updated = "2026-10-15T10:04:19Z"
scope = ["docs/spec/adapters.md"]

[[acceptance]]
text = "adapters.md no longer says a backend without a Document answers Backend::push"
bound = true
+++

Found while working ~5SFKGWW. docs/spec/adapters.md lines ~128-131 say a backend without a Document still answers Backend::push. Backend::push is removed; every backend exposes Backend::document and sync pushes via push_atomic. Reword that paragraph.
