+++
id = "01M4V1HEJVPSN7XKGGXJ3R5FA0"
title = "Spec: quantities.md ambiguities found writing fixtures"
type = "bug"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-13T16:29:12Z"
updated = "2026-10-16T07:13:04Z"
idempotency_key = "Z48-ambig"
scope = ["docs/spec/quantities.md"]
+++

found while working ~Z48GXKX. Unspecified: (1) code for a reversed range 'x in 6mm..4mm' (impl EVAL); (2) code for reading an absent bound limit; (3) Count times anonymous scalar '2 * 0.5' 4.1.6 vs 4.4; (4) mixed-pi addition inexact vs error (section 1 vs 5.4); (5) E009 syntax: enum is top-level; (6) 'satisfies <= 8.5mm' is E001 though the text reads as allowed; (7) divide by zero is internal EVAL code, not listed. Decided and written into spec: bare nonzero tolerance deviation Q001; affine multiply/divide Q002; display conformance surface.
