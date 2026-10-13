+++
id = "01M4ST8EJWTGQM9CMVBJVBATJT"
title = "Obligation graph and lock file (cadetta-obligations)"
type = "epic"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T05:02:43Z"
updated = "2026-10-13T05:05:10Z"
idempotency_key = "replan13:Obligation graph and lock file (cadetta-obligations)"
labels = ["creates:crates/cadetta-obligations/**"]
scope = ["crates/cadetta-obligations/**"]

[[links]]
kind = "blocks"
target = "01M4STAMTNCTC1MRHE5VYP9QYN"
+++

Outcome: every claim cadetta makes is an obligation pinned to Merkle closure digests; staleness is computed from the graph plus cadetta.lock, per docs/spec/obligations.md. Core functionality.

Success metric: changing a value upstream of an attested check makes exactly the covering attestations stale, and why/impact answer with the chain.
