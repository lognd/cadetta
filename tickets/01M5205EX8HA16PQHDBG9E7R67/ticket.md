+++
id = "01M5205EX8HA16PQHDBG9E7R67"
title = "Import executor: then_sync/design/native/lock modelling and fixture inconsistencies"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-16T09:19:52Z"
updated = "2026-10-16T11:20:35Z"
scope = ["crates/cadetta-cli/tests/**"]

[[acceptance]]
text = "The four formerly gapped import fixtures (contract-no-source-write, import-never-overwrites, import-update-next-sync-no-op, resolve-drift-import-update) pass in the adapters corpus with their known-gaps entries removed"
bound = false
+++

Found while working ~KKVE13B. import-update-next-sync-no-op and resolve-drift-import-update need then_sync/next_sync_class/design/native/lock modelling. import-never-overwrites contradicts import-propose-writes-proposal accept not-fire (same accept inputs; one expects writes [bindings, cdt], the other only [cdt]); contract-no-source-write not-fire expects writes_source true with no source (unsatisfiable). Resolve the fixtures, then model.
