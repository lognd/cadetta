+++
id = "01M55E3PRRE8344MJC1YMEEPJ0"
title = "import --update writes a SOLIDWORKS unitless count (pattern count) as 4mm: empty adapter unit means assume native"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T17:21:18Z"
updated = "2026-10-17T17:27:25Z"
scope = ["crates/cadetta-adapter/src/import/**", "crates/cadetta-adapter/src/read_units.rs"]
+++

Found live on nova under ~W5MBBTB. The SW adapter reads D1@LPattern1 as text 4 with unit empty (Count kind). sync compares it fine (4 -> 4), but 'cadetta import solidworks <part> --update --write' proposes and writes bolt_count = 4mm into lib.cdt, because an empty/absent wire unit resolves to the adapter native unit (mm). Needs a way for an adapter to say dimensionless (e.g. explicit unit '1') or the import writer to honour the param's declared type. Evidence: adapters/solidworks/evidence/w5mb-live/run5.log.
