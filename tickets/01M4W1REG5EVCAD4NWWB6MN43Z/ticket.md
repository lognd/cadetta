+++
id = "01M4W1REG5EVCAD4NWWB6MN43Z"
title = "Call geometry_bind verify_geometry and datum_entries from cadetta sync"
type = "task"
category = "todo"
priority = "high"
reporter = "lognd"
created = "2026-10-14T01:52:16Z"
updated = "2026-10-14T20:47:56Z"
scope = ["crates/cadetta-cli/src/cmd/sync.rs", "docs/spec/adapters.md", "docs/spec/config-cli.md"]

[[links]]
kind = "blocked-by"
target = "01M4SPWTDD069AXXQBXD4T6ATZ"
+++

found while working 645FHJZ: no cadetta sync verb exists yet (owned by ~D4T6ATZ). 645FHJZ landed the glue as cadetta_adapter::geometry_bind::{Composer, verify_geometry, datum_entries}; the verb must call verify_geometry after read (R001 diagnostics) and append datum_entries to the sync entries when datums = create. Also needs the unknown-verb test in lib.rs changed off 'sync'.
