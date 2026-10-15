+++
id = "01M4YTGQ9BHMKS38GTGX2PFH03"
title = "conformance: adapters/bind-scope-every-in-scope-path exits 2, expected 0"
type = "bug"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T03:43:26Z"
updated = "2026-10-15T04:11:07Z"
idempotency_key = "gap-bind-scope-every"
scope = ["crates/cadetta-adapter/**", "crates/cadetta-cli/src/**"]

[[acceptance]]
text = "adapters/bind-scope-every-in-scope-path passes and its gap entry is removed"
bound = true
+++

Found while cleaning known-gaps (~YE4KW2A): check exits 2 on a fixture that must pass cleanly.
