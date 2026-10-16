+++
id = "01M52TH35NQBTKBHMBH6WF08GT"
title = "ack: accept sync:FILE as an attestable obligation"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-16T17:00:36Z"
updated = "2026-10-16T17:22:16Z"
scope = ["crates/cadetta-cli/src/cmd/ack.rs", "crates/cadetta-cli/src/universe.rs"]
+++

found while working ~FEZDG87. universe::sync_states now honours an Attest record on sync:FILE, but cadetta ack has no candidate for sync obligations.
