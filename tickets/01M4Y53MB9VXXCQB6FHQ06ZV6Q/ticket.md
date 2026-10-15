+++
id = "01M4Y53MB9VXXCQB6FHQ06ZV6Q"
title = "Load upstream locks of git dependencies in cadetta obligations"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-14T21:29:17Z"
updated = "2026-10-15T10:04:16Z"
idempotency_key = "sqrbg5c-git-upstream"
scope = ["crates/cadetta-cli/src/cmd/obligations.rs", "crates/cadetta-cli/src/pkg/**", "crates/cadetta-cli/src/cmd/update.rs"]

[[acceptance]]
text = "A git dependency's cadetta.lock is materialised (resolver Io.fetch) and its attestations show as attested upstream in cadetta obligations"
bound = true

[[acceptance]]
text = "trust roles on a git dependency discharge matching dependent obligations"
bound = true

[[acceptance]]
text = "Candidate obligations include dependency nodes in the analysis universe so upstream matching by ObligationId is exercised end to end"
bound = false
+++

Found while working ~SQRBG5C: upstream_of() reads only path dependencies; git deps are skipped with a debug log. pkg::resolve is not yet wired into the CLI, and it is unverified that dependency nodes appear in the candidate obligation universe.
