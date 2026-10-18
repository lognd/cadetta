# Cadetta specification

Status: draft 1.5 (2026-10-13), revised after the pilot rounds, the final review round (`docs/history/`) and the owner's citation, placement and SOLIDWORKS revision (`docs/decisions/2026-10-13-draft-1-5-decisions.md`). Normative unless a section says otherwise.

Cadetta is the single source of truth for the parameters of a CAD design.
Engineers author parameters once, as typed quantities with provenance and
tolerances, in `.cdt` source files. Cadetta lowers them into a
tool-agnostic intermediate representation (IR), lints the IR, and moves
values in and out of CAD tools through adapters so nobody retypes a
dimension by hand.

```
  .cdt source --parse/resolve/eval--> design IR --+--> lint --> diagnostics
                                                  |
  CAD files  --adapter read-------->  observed IR-+--> drift lint
                                                  |
  design IR  --adapter emit/drive---> CAD params, regen, read-back
```

## Documents

| Document | Covers |
|---|---|
| [quantities.md](quantities.md) | Quantities, units, kinds, tolerances, exactness. First-class; read first. |
| [language.md](language.md) | Surface syntax of `.cdt` files and its static semantics. |
| [ir.md](ir.md) | The tool-agnostic IR: design IR, observed IR, canonical paths, digests. |
| [adapters.md](adapters.md) | The emit / read / drive adapter contract and the CAD target matrix. |
| [obligations.md](obligations.md) | The obligation graph: digests, staleness, attestations, why/impact/swap queries. Core. |
| [solvers.md](solvers.md) | External solvers (Python, MATLAB, ...): protocol, caching, staleness. |
| [packages.md](packages.md) | Packages, modules, visibility (`pub`), `use`, dependencies and their obligations. |
| [stdlib.md](stdlib.md) | The standard library: fits, materials, fasteners and `ScrewJoint`, retaining rings; table columns; shop policy. |
| [emitters/](emitters/sw-equations.md) | One format document per built-in emitter: [json](emitters/json.md), [csv](emitters/csv.md), [sw-equations](emitters/sw-equations.md). |
| [lints.md](lints.md) | The rule catalog: codes, severities, waivers. |
| [config-cli.md](config-cli.md) | `cadetta.toml` and the `cadetta` command line. |
| [geometry.md](geometry.md) | Positions and datums: datum-typed points, placement trees, alignment checks. |
| [bindings.md](bindings.md) | Language bindings: the JSON catch-all, the per-language layout, and the Python package. |
| [solidworks.md](solidworks.md) | The SOLIDWORKS adapter and add-in: workflow, linking by selection, native mapping, testing. |
| [fusion.md](fusion.md) | The Fusion adapter. |
| [ansys.md](ansys.md) | The Ansys adapter (specification only; no implementation yet). |

Decisions are recorded in `docs/decisions/`; architecture is modeled in
`design/model.grmb` (grimble) and work is tracked as frob tickets.

## Problems this solves (from the README)

| Problem | Mechanism | Rules |
|---|---|---|
| Duplication: one OTS dimension drives many parts | Values are referenced, never restated; derived values are expressions over their sources | D001-D003, E001-E011 |
| Naming: "diameter" everywhere, uncanonical prefixes | Names are scoped by canonical paths (`pkg::module::Component.param`); case and form are linted; adapters map native names to canonical ones | N001-N006 |
| Magic numbers: is 12.012mm a mistake or a fit? | Tolerances and ISO 286 fits are values, not comments; unexplained precision is flagged | M001-M003, F001-F004 |
| Where did this number come from, and can I find it again? | Typed IEEE-style citations with revisions and pinned files; a changed source flags every value cited to it (S004); `cite list` prints the reference list | S001, S002, S004, S008 |
| Manual retyping into CAD | Adapters emit, read and drive CAD tools, links made in CAD count as bindings, tolerances are written too; drift is linted | R001-R016 |
| Changing a part (cost, lead time, end of life, performance) | Interfaces, approved alternates verified continuously, swap previews | I, L, `impact --swap` |
| Copying a shop policy or parts library between projects | Packages with `pub` APIs, versioned dependencies, lock-pinned digests | packages.md |
| Hand-computing holes, tap drills and screw lengths | `std::fasteners::ScrewJoint` derives them from thread, clamp and material | stdlib.md 3 |
| Where features sit, and whether holes line up across parts | Datum-typed positions, true position zones, placement trees verified against CAD transforms, `aligned`, joint float | G001-G008 |
| Knowing what a change invalidates | The obligation graph pins every claim to digests | O001-O004, S003, S004 |

## Open questions

Each `OQ-n` lives at the end of its document. Status as of this draft.

| Id | Document | Status |
|---|---|---|
| OQ-1 | quantities.md | closed 2026-10-13 (README) |
| OQ-2 | quantities.md | open |
| OQ-3 | quantities.md | closed 2026-10-13 (language.md 3.10) |
| OQ-4 | language.md | deferred past 0.1.0 |
| OQ-5 | language.md | closed 2026-10-13 (language.md 2) |
| OQ-6 | language.md | open |
| OQ-7 | adapters.md | closed 2026-10-13 (adapters.md 5) |
| OQ-8 | adapters.md | closed 2026-10-13 (adapters.md 2.2) |
| OQ-9 | quantities.md | open |
| OQ-10 | geometry.md | closed 2026-10-13 (geometry.md) |
| OQ-11 | language.md | closed 2026-10-13 (packages.md) |
| OQ-12 | obligations.md | closed 2026-10-13 (obligations.md 3) |
| OQ-13 | obligations.md | open |
| OQ-14 | stdlib.md | open |
| OQ-15 | stdlib.md | closed 2026-10-13 (draft 1.3 decisions) |
| OQ-16 | bindings.md | closed 2026-10-13 (bindings.md 3.1) |
| OQ-17 | geometry.md | narrowed 2026-10-13 (geometry.md 3.1); MMC open |
| OQ-18 | geometry.md | open |
| OQ-19 | packages.md | open |
| OQ-20 | packages.md | closed 2026-10-13 (packages.md 3) |

## Scope of 0.1.0

In: the `.cdt` language (including interfaces, alternates, tables,
parameterized components and processes), the quantity system, positions
and datums (geometry.md), packages, modules and visibility (packages.md), the metric standard
library (stdlib.md 1),
the IR and its JSON schema, the obligation graph and lock file, the
solver protocol, the lint catalog, built-in file emitters (JSON, CSV,
SOLIDWORKS linked-equations file), the adapter protocol, the SOLIDWORKS
adapter and add-in (C#, read and drive, the primary target;
solidworks.md), the FreeCAD adapter (the CI reference), the Python
binding (bindings.md), and `cadetta lsp` for editors (diagnostics and
formatting, config-cli.md 3.8).

Out (each recorded as a dropped or deferred ticket with its reason):
tolerance stack analysis (RSS or statistical), constraint solving,
geometry kernels and mate solving, GD&T beyond true position regardless
of feature size (OQ-17), CAD
configurations / design-table variants, electrical CAD (KiCad), a GUI,
and a package registry (OQ-19).

## Conventions

- "MUST", "SHOULD", "MAY" are used as in RFC 2119.
- Open questions are marked `OQ-n`, collected at the end of each
  document and indexed above. An open question is never silently
  resolved: it closes with an ADR in `docs/decisions/` or a dated
  statement in its document (`closed DATE (ADR or doc)`).
