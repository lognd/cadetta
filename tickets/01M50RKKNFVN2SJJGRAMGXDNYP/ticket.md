+++
id = "01M50RKKNFVN2SJJGRAMGXDNYP"
title = "S003 stale messages, stale_output observation, C001/C002 from solver outputs in check"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T21:48:32Z"
updated = "2026-10-15T23:21:59Z"
scope = ["crates/cadetta-cli/src/**,crates/cadetta-cli/tests/**,tests/conformance/known-gaps.toml", "crates/cadetta-sema/src/checks.rs", "crates/cadetta-cli/tests/**", "docs/spec/solvers.md", "tests/conformance/obligations/s003-solver-stale.expect.json", "crates/cadetta-sema/src/eval.rs", "crates/cadetta-cli/src/pipeline.rs", "tests/conformance/known-gaps.toml"]

[[acceptance]]
text = "S003 message names what changed (dep path, inputs, script)"
bound = true

[[acceptance]]
text = "Conformance observes stale_output for a stale solver whose outputs still evaluate"
bound = true

[[acceptance]]
text = "A check inside a solver on its own recorded output yields C001"
bound = true

[[acceptance]]
text = "A check reading an unsolved output yields C002 and not S003"
bound = true
+++

Split from ~B645GTM. Fixtures solvers/s003-stale-{dep,inputs,script,still-evaluates}, trust-solver-own-check, unsolved-output-no-result still fail with S006 landed: S003 messages lack the changed name (analysis/material_db.csv, inputs, script); s003-stale-still-evaluates does not observe stale_output; C001 (trust-solver-own-check) and C002 (unsolved-output-no-result) are not produced. Make them pass and remove their known-gaps entries.
