+++
id = "01M4YG7D22XEEK9M3X678B88BT"
title = "conformance_corpus_matches_the_spec hangs: a corpus adapter command receives the initialize JSON as its shell command"
type = "bug"
category = "todo"
priority = "high"
reporter = "lognd"
created = "2026-10-15T00:43:35Z"
updated = "2026-10-15T02:30:08Z"
scope = ["crates/cadetta-cli/tests/conformance.rs", "crates/cadetta-cli/src/cmd/check_drift.rs", "crates/cadetta-adapter/src/rpc/client.rs", "crates/cadetta-cli/src/cmd/import.rs", "docs/spec/adapters.md", "changelog.d/01M4YG7D22XEEK9M3X678B88BT.fixed.md"]

[[acceptance]]
text = "conformance_corpus_matches_the_spec completes"
bound = true

[[acceptance]]
text = "A non-answering adapter makes check report R008 within a bounded time"
bound = false
+++

Found while working ~A8J3XJW; reproduced at fc17de73 (before that ticket). After the drift lints were wired into check (~3RK3GPH), the conformance corpus run prints 'sh: 1: {id:1,jsonrpc:2.0,method:initialize,...}: not found' and never finishes (cargo test -p cadetta --test conformance conformance_corpus). Some corpus fixture configures an adapter command that is spawned through sh with the request as the command. Find the fixture, fix the spawn or the fixture, and bound the adapter read with a timeout so a bad adapter cannot hang check.
