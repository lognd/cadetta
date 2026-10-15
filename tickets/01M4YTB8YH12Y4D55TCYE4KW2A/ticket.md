+++
id = "01M4YTB8YH12Y4D55TCYE4KW2A"
title = "Clean stale conformance known-gaps entries"
type = "chore"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T03:40:27Z"
updated = "2026-10-15T03:46:22Z"
idempotency_key = "known-gaps-cleanup"
scope = ["tests/conformance/**", "changelog.d/**"]

[[acceptance]]
text = "conformance_corpus_matches_the_spec passes"
bound = false
+++

Remove known-gaps entries for fixtures that now pass; retarget genuine failures.
