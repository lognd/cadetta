+++
id = "01M4VA52FE3C93RRPS978T98Y4"
title = "Spec draft 1.5: IEEE citations, placement revision, position tolerance, SOLIDWORKS workflow"
type = "docs"
category = "todo"
priority = "high"
points = 8
reporter = "lognd"
created = "2026-10-13T18:59:44Z"
updated = "2026-10-13T18:59:44Z"
scope = ["docs/spec/**", "docs/decisions/**"]

[[acceptance]]
text = "Given docs/spec, when read, then citations are typed IEEE-style records with per-type locating fields, S008 flags an incomplete one, and solver scripts are no longer citations"
bound = false

[[acceptance]]
text = "Given geometry.md, when read, then placement is 'place A at B' with elementwise array placement, op paths match datum op paths, the bolt float applies to any parent-child joint edge with a rotational term, and RFS position tolerance is specified"
bound = false

[[acceptance]]
text = "Given adapters.md and solidworks.md, when read, then mangling is a pure per-path function, native links are implicit bindings, and the SOLIDWORKS workflow (C# adapter and add-in, frames, placements, tolerances) is specified"
bound = false

[[acceptance]]
text = "Given docs/decisions, when read, then a draft 1.5 ADR records every decision and the superseded SOLIDWORKS-in-Python decision is marked"
bound = false
+++

Owner-requested revision (2026-10-13). Citations become typed IEEE-style records; placement verb, elementwise array placement, unified op paths and a sound bolt float; RFS true position on points; mangling becomes a pure per-path function; native links (mangled globals, dimension refs, coordinate systems, component references) count as bindings; the SOLIDWORKS adapter moves to C# with an add-in. Spec and ADR only; implementation is in child tickets.
