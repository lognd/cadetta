+++
id = "01M52QZA3CBFV6NMGVXTSTDBAZ"
title = "Conformance: sync entry model for attach/close, r006 dry-run, degrade and header-noop fixtures"
type = "task"
category = "todo"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-16T16:15:56Z"
updated = "2026-10-16T16:36:15Z"
scope = ["crates/cadetta-cli/tests/**", "tests/conformance/**"]
+++

found while working ~HMB7P7J. The tool-session sync executor (tests/conformance sync.rs) covers R004, R008-R010, save, adopted, R003, tol. Still gapped: adapters/session-attach-running and session-close-only-opened (adapter-internal attach/close; need an adapter-side harness, a Sim would be tautological), r006-dimension-equation-removed (needs a dry-run diff checker, dry_run_shows), sync-degrade-no-theirs-no-r001 (sync::run emits R001 without theirs; classified observation) and sync-degrades-without-drive (classified/emitted/instructions observations), and emitters/*/sync-header-excluded-from-noop (existing/emitted text shape). Also open question: sync::run emits R001 under --force-design while adapters.md 2 says it pushes over it (executor filters it).
