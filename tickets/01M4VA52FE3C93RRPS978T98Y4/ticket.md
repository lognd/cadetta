+++
id = "01M4VA52FE3C93RRPS978T98Y4"
title = "Spec draft 1.5: IEEE citations, placement revision, position tolerance, SOLIDWORKS workflow"
type = "docs"
category = "done"
outcome = "done"
priority = "high"
points = 8
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T18:59:44Z"
updated = "2026-10-13T20:37:20Z"
labels = ["creates:docs/spec/solidworks.md"]
scope = ["docs/decisions/**", "docs/spec/README.md", "docs/spec/language.md", "docs/spec/geometry.md", "docs/spec/ir.md", "docs/spec/adapters.md", "docs/spec/lints.md", "docs/spec/obligations.md", "docs/spec/config-cli.md", "docs/spec/stdlib.md", "docs/spec/bindings.md", "docs/spec/solvers.md", "docs/spec/quantities.md", "docs/spec/emitters/sw-equations.md", "docs/spec/solidworks.md", "design/model.grmb", "design/vmodel.grmb", "docs/decisions/2026-10-13-bindings-and-solidworks.md"]

[[acceptance]]
text = "Given docs/spec, when read, then citations are typed IEEE-style records with per-type locating fields, S008 flags an incomplete one, and solver scripts are no longer citations"
bound = true

[[acceptance]]
text = "Given geometry.md, when read, then placement is 'place A at B' with elementwise array placement, op paths match datum op paths, the bolt float applies to any parent-child joint edge with a rotational term, and RFS position tolerance is specified"
bound = true

[[acceptance]]
text = "Given adapters.md and solidworks.md, when read, then mangling is a pure per-path function, native links are implicit bindings, and the SOLIDWORKS workflow (C# adapter and add-in, frames, placements, tolerances) is specified"
bound = true

[[acceptance]]
text = "Given docs/decisions, when read, then a draft 1.5 ADR records every decision and the superseded SOLIDWORKS-in-Python decision is marked"
bound = true
+++

Owner-requested revision (2026-10-13). Citations become typed IEEE-style records; placement verb, elementwise array placement, unified op paths and a sound bolt float; RFS true position on points; mangling becomes a pure per-path function; native links (mangled globals, dimension refs, coordinate systems, component references) count as bindings; the SOLIDWORKS adapter moves to C# with an add-in. Spec and ADR only; implementation is in child tickets.
