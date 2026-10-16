+++
id = "01M52QZA3CBFV6NMGVXTSTDBAZ"
title = "Conformance: sync entry model for attach/close, r006 dry-run, degrade and header-noop fixtures"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-16T16:15:56Z"
updated = "2026-10-16T16:58:19Z"
scope = ["crates/cadetta-cli/tests/**", "tests/conformance/**", "docs/spec/adapters.md"]

[[acceptance]]
text = "r006-dimension-equation-removed passes: dry-run shows the equation removal before the set, R006 warning, nothing written"
bound = true

[[acceptance]]
text = "sync-degrade-no-theirs-no-r001 and sync-degrades-without-drive pass: no drive degrades to emit plus instructions with no classification and no R001"
bound = false

[[acceptance]]
text = "the csv, json and sw-equations sync-header-excluded-from-noop fixtures pass: a header-only difference writes nothing"
bound = false

[[acceptance]]
text = "session attach and close-only-opened fixtures stay gapped with a filed follow-up (~6NNC66V); R001 under --force-design decided and documented"
bound = false
+++

found while working ~HMB7P7J. The tool-session sync executor (tests/conformance sync.rs) covers R004, R008-R010, save, adopted, R003, tol. Still gapped: adapters/session-attach-running and session-close-only-opened (adapter-internal attach/close; need an adapter-side harness, a Sim would be tautological), r006-dimension-equation-removed (needs a dry-run diff checker, dry_run_shows), sync-degrade-no-theirs-no-r001 (sync::run emits R001 without theirs; classified observation) and sync-degrades-without-drive (classified/emitted/instructions observations), and emitters/*/sync-header-excluded-from-noop (existing/emitted text shape). Also open question: sync::run emits R001 under --force-design while adapters.md 2 says it pushes over it (executor filters it).
