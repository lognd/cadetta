+++
id = "01M4ZAN1F6F1R2DFW1PHMB7P7J"
title = "Conformance: tool-session model for binding sync fixtures (R004 R008-R010 save attach O004)"
type = "task"
category = "in-progress"
priority = "medium"
points = 8
reporter = "lognd"
created = "2026-10-15T08:25:25Z"
updated = "2026-10-16T16:16:57Z"
scope = ["crates/cadetta-cli/tests/**", "tests/conformance/**"]

[[acceptance]]
text = "Binding sync fixtures decided by the tool-session model (host faults, R004, R008-R010, save, adopted) execute and are compared and their known-gaps entries are removed; the rest are retargeted to ~FEZDG87 and ~TSTDBAZ."
bound = false
+++

Split from ~CDJGJMG. The in-process sync loop executor (tests/conformance sync.rs) covers fixtures decided by sync::run; these need the host session logic in cmd/sync.rs driven through scripted adapters (protocol.rs). Also: sync::run emits R001 under --force-design although adapters.md 2 says it pushes over it; the executor filters it, library should decide.
