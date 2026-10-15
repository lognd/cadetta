+++
id = "01M4YTGQBH5YD50C2ECZHXK8Z8"
title = "conformance: spurious S008 warning on sourced-component fixtures (stdlib i001/q003)"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T03:43:26Z"
updated = "2026-10-15T04:28:04Z"
idempotency_key = "gap-s008-noise"
scope = ["crates/cadetta-cli/src/**", "crates/cadetta-core/**"]

[[acceptance]]
text = "the three stdlib fixtures pass and their gap entries are removed"
bound = true
+++

Found while cleaning known-gaps (~YE4KW2A): extra S008 warning at acme.demo::src in stdlib/i001-sourced-optional-members, i001-sourced-wrong-kind, q003-sourced-named-kind.
