+++
id = "01M4Z0WGJPRDATAN7V7N8B2SV9"
title = "geometry: unknown datum of a placed part must report E002, not EVAL"
type = "bug"
category = "todo"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T05:34:44Z"
updated = "2026-10-15T05:34:44Z"
idempotency_key = "datum-unknown-placed-e002"
scope = ["crates/**"]

[[acceptance]]
text = "Fixture geometry/datum-unknown-of-placed-part passes with E002 and no EVAL"
bound = false
+++

Fixture geometry/datum-unknown-of-placed-part: 'a.pt in a.nothing' in an assembly check yields EVAL at acme.demo::A.c instead of E002.
