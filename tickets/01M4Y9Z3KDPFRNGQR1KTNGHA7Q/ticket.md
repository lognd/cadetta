+++
id = "01M4Y9Z3KDPFRNGQR1KTNGHA7Q"
title = "Enumerate fit, require, sync, solver and waiver obligations; read-member deps for check closures"
type = "task"
category = "done"
outcome = "done"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-14T22:54:12Z"
updated = "2026-10-15T10:38:46Z"
scope = ["crates/cadetta-cli/src/universe.rs", "crates/cadetta-cli/src/cmd/obligations.rs", "crates/cadetta-ir", "crates/cadetta-obligations", "crates/cadetta-ir/**", "docs/spec/obligations.md"]

[[links]]
kind = "blocked-by"
target = "01M4TA3C5QWZ2NH70V6QN4PR7X"

[[acceptance]]
text = "Given a component with a fit that has a required bound, when 'cadetta obligations' runs, then fit:PATH is listed and judged against the fit digest"
bound = true

[[acceptance]]
text = "Given a solver, sync binding or waiver in the design, when 'cadetta obligations' runs, then solver:, sync: and waiver: ids are listed with their closure digests"
bound = true

[[acceptance]]
text = "Given a check that reads a table or part argument, when that input is edited, then the check's attestation is stale"
bound = true
+++

Split from ~QN4PR7X, which now delivers ots and citation enumeration. Remaining: obligations/ack candidates for fit (recomputed from Fit.required vs clearance), require, sync, solver and waiver kinds; S003/S004/O004 as enumerated facts; check closures lacking read-member deps beyond the textual check_reads rule (tables, part args). The IR has no require node yet, which may need an IR ticket first.
