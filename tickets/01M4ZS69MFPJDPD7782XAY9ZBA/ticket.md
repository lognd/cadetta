+++
id = "01M4ZS69MFPJDPD7782XAY9ZBA"
title = "Wire initialize precision into CLI call sites via Client::effective_precision"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T12:39:30Z"
updated = "2026-10-15T16:07:27Z"
scope = ["crates/cadetta-cli/**", "docs/spec/adapters.md"]

[[acceptance]]
text = "cadetta sync uses the precision reported by initialize unless [[adapter]] precision is set"
bound = true
+++

found while working ~6N3JQAV: universe.rs:501, cmd/sync.rs (661/925/1153), cmd/names.rs:155, cmd/emit.rs:508 still use adapter.precision.map_or(DEFAULT_PRECISION, ...) instead of the initialize precision. cmd/sync.rs is owned by in-flight ~5HP91AE; sequence after it.
