# Draft 1.5: citations, placement and the SOLIDWORKS workflow

Status: current
Owner: cadetta
Decisions: none
Audience: contributors

- Date: 2026-10-13
- Ticket: ~78T98Y4

## Context

The owner asked for three things after draft 1.4: citations that work
like a bibliography (typed, importable, IEEE-shaped, so a reader can
find the source again); an audit of placement semantics; and a
SOLIDWORKS workflow that does not make the engineer say everything
twice, possibly through a C# add-in that turns a selected dimension
into its mangled name. An audit of draft 1.4 found:

- A `cite` string that is not a URL was a file path, so every free-text
  `cite` in the golden corpus (cases 04 and 08) named a file that does
  not exist; an inline `@"quote ..."` was filed as kind `standard`; a
  solver script was written as a citation.
- `place A on B` made frames equal, but "on" reads as face contact,
  whose in-plane orientation is underdetermined (flip about x or y?).
  The grammar admitted only `for i in 0..count(P)` while G006 and G007
  spoke of partial and overlapping ranges, and `..` was exclusive there
  and inclusive in bounds. Placement op paths (`place.0.x`) used a
  different scheme from datum op paths (`offset[1].x`), so inserting
  an op renumbered bindings. `as Point2 on D` appeared in prose only.
- The bolt float attached only when the `place` target was built from
  the datum `onto` is declared on, so placing the same part from a
  sibling datum silently dropped the float; rotation inside the
  clearances was ignored, so a check far from the pattern could hold
  when the part can in fact swing out of it.
- Mangling qualified colliding names with their module, so adding a
  path could rename an existing global and break every SOLIDWORKS
  dimension linked to it.
- Every bound path needed a bindings-file entry restating a name that
  the mangling rule already determines, and a SOLIDWORKS dimension
  linked to a cadetta global needed a second entry recording a link the
  part file already holds.

## Decision

1. **Citations are typed records modelled on IEEE reference types**
   (language.md 3.9). `cite NAME: TYPE { field = "..." ... }` with types
   `standard datasheet drawing catalog book article web quote document`
   and IEEE fields (`author organization title number rev edition date
   publisher location journal volume issue pages doi isbn url accessed
   file note`). Each type names the fields that make the source
   findable again; a missing one is S008. The short `cite x = "..."`
   and `cite x = std::...` forms are removed: a standard is cited as
   `@std::iso273` or imported with `use`, a note is an inline
   `@"text"` (kind `note`). A citation's digest covers its fields and
   file, so bumping a datasheet's `rev` makes its acknowledgement stale
   and S004 lists every value cited to it; values themselves keep their
   digests, so no review fires twice. `cadetta cite list` renders an
   IEEE reference list (or BibLaTeX, or JSON); `cite pin` keeps the URL
   and adds `file`.
2. **Solver scripts are inputs, not citations**: `solver NAME { script
   "path" ... }` (solvers.md 1).
3. **`place A at B` makes datum A's frame equal B's.** The verb no
   longer suggests contact. A datum is a frame, not a face; the
   convention for mounting datums is that z points along the stack, out
   of the host and into the mounted part, which is what a part modelled
   from its base face gets for free (geometry.md 2, 4).
4. **Arrays place elementwise**: `place feet.top at
   base.underside.offset(base.foot_sites)` places instance i at point
   i; counts must agree (G004). The `for` clause on `place` and the
   `.at(point)` op are removed; `.offset(point)` is the point form of
   `offset`. Exclusive `..` no longer exists in the language.
5. **Placement op paths use the datum scheme**: `Mount/bracket.place.offset.x`,
   `place.offset[1].x`, `place.rotate.z`.
6. **The bolt float attaches to every parent-child joint edge**, whatever
   datum the placement names, and carries a rotational term about the
   `onto` pattern's centroid: `2 * asin(float / span)`, unbounded for a
   one-point pattern (G008). Translation stays `[+-float]` per axis.
   Both bounds are sound.
7. **RFS true position is in 0.1.0** (geometry.md 3.1, quantities.md
   5.4). `[position 0.2mm]` after a `Point2` or a pattern gives each
   point one disc-shaped source variable of diameter 0.2mm. `aligned`
   over position zones is exact up to the final square root. Material
   condition modifiers (MMC bonus) stay OQ-17.
8. **Pattern counts bind.** `holes.count_x`, `holes.count_y`,
   `holes.count` are members that emit to a native pattern's instance
   count; they stay structural (exact, known before evaluation).
9. **Mangling is a pure function of the path** (adapters.md 3.1). A
   collision is R013, resolved with an explicit binding or
   `#[native(...)]`; nothing is ever renamed by an unrelated addition.
10. **Native links are bindings** (adapters.md 3.5). A native global
    whose name is the mangled name of an in-scope path, a dimension
    whose native equation is exactly a reference to such a global, a
    coordinate system named by a datum's mangled path, and a component
    whose reference property names a part path are bound without a
    bindings-file entry. The file holds only exceptions (legacy names,
    `value` mode, `for`, `config`, `sign`). `[[adapter]] bind = "scope"`
    additionally binds every in-scope path to its mangled global, which
    is how an equations-file user gets globals to link to.
11. **Positions are verified by transform, driven by datum.** A datum
    maps to a coordinate system feature, which `sync` can create and
    drive from the composed transform (`[[adapter]] datums = "create"`);
    a placement is verified by comparing the component's transform
    with the composed placement (R001 on drift). The idiom that removes
    duplication: put every offset in a host datum, place op-free, and
    mate coordinate systems coincident in CAD; then no mate carries a
    number and nothing is bound by hand.
12. **Tolerances drive dimension tolerances.** A bound SOLIDWORKS
    dimension receives the path's tolerance (bilateral, limit, or ISO
    fit class) unless `[[adapter]] tolerances = "ignore"`; read-back is
    compared like the value.
13. **The SOLIDWORKS adapter is C#**, one solution in
    `adapters/solidworks/` with a protocol executable (read, drive) and
    a SOLIDWORKS add-in (selection-driven linking). The add-in never
    computes a name or edits a bindings file itself: it asks
    `cadetta names` and calls `cadetta bind`, so mangling and the file
    format have one implementation (solidworks.md). This supersedes
    decision 5 of the bindings-and-solidworks ADR (Python on pywin32).

## Consequences

- Array placements lower to one placement per instance with integer
  indexes; the IR no longer has range variables.
- The parser, sema, IR, digests, lints (S001, S002, S004, S008, G004,
  G006, G007, G008, R001, R013), `cite pin`, mangling, and the geometry
  crate change; the std citations, the golden corpus and the citation
  and geometry conformance fixtures are rewritten with them.
- The Python SOLIDWORKS tickets are replaced by C# tickets. CI builds
  and tests the C# code that does not reference the SOLIDWORKS interop
  assemblies; the shell that does builds on the owner's machine.
- New CLI verbs: `cite list`, `names`, `bind`.
- OQ-17 narrows to material condition modifiers.

## Alternatives considered

- **Two placement verbs** (`on` for face contact with opposed normals,
  `at` for coincident frames). Rejected: opposing z leaves the in-plane
  orientation to an arbitrary choice of flip axis, and CAD coordinate
  system mates are coincident. One verb plus a stacking convention is
  unambiguous.
- **Reading `.bib` files at compile time.** Rejected: it adds a second
  source of truth. BibTeX is an export format; import, if wanted, is a
  proposal like `cadetta import` (deferred).
- **Keeping the short string `cite` form as a note.** Rejected: two
  spellings for an unstructured note, and a named note is exactly what
  should carry fields.
- **A Python SOLIDWORKS adapter plus a C# add-in.** Rejected: two code
  bases against one COM API. An in-process add-in must be .NET, so the
  adapter joins it.
- **Binding every in-scope path implicitly by default.** Rejected for
  0.1.0: it floods a part with globals and changes the meaning of every
  existing bindings fixture. It is one config line (`bind = "scope"`).
- **Creating mates from `place`.** Deferred: verification by transform
  covers correctness without cadetta owning mate topology; an add-in
  command can create the coincident coordinate-system mate later.
