+++
id = "01M52TH35NQBTKBHMBH6WF08GT"
title = "ack: accept sync:FILE as an attestable obligation"
type = "task"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-16T17:00:36Z"
updated = "2026-10-16T17:21:50Z"
scope = ["crates/cadetta-cli/src/cmd/ack.rs"]
+++

found while working ~FEZDG87. universe::sync_states now honours an Attest record on sync:FILE, but cadetta ack has no candidate for sync obligations.
