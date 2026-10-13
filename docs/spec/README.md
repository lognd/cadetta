# Cadetta specification

Status: draft 0 (2026-10-12). Normative unless a section says otherwise.

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
| [lints.md](lints.md) | The rule catalog: codes, severities, waivers. |
| [config-cli.md](config-cli.md) | `cadetta.toml` and the `cadetta` command line. |

Decisions are recorded in `docs/decisions/`; architecture is modeled in
`design/model.grmb` (grimble) and work is tracked as frob tickets.

## Problems this solves (from the README)

| Problem | Mechanism | Rules |
|---|---|---|
| Duplication: one OTS dimension drives many parts | Values are referenced, never restated; derived values are expressions over their sources | D001-D003 |
| Naming: "diameter" everywhere, uncanonical prefixes | Names are scoped by canonical paths (`ns::Component.param`); case and form are linted; adapters map native names to canonical ones | N001-N004 |
| Magic numbers: is 12.012mm a mistake or a fit? | Tolerances and ISO 286 fits are values, not comments; unexplained precision is flagged | M001-M003, F001-F003 |
| Manual retyping into CAD | Adapters emit, read and drive CAD tools; drift is linted | R001-R003 |

## Scope of 0.1.0

In: the `.cdt` language, the quantity system, the IR and its JSON
schema, the lint catalog, built-in file emitters (JSON, CSV, SOLIDWORKS
linked-equations file), the adapter protocol, and one reference external
adapter that can read and drive (FreeCAD; headless, testable in CI).

Out (each recorded as a dropped or deferred ticket with its reason):
tolerance stack analysis (RSS or statistical), constraint solving,
geometry kernels, GD&T, CAD configurations / design-table variants,
electrical CAD (KiCad), and a GUI.

## Conventions

- "MUST", "SHOULD", "MAY" are used as in RFC 2119.
- Open questions are marked `OQ-n` and collected at the end of each
  document. An open question is never silently resolved: it closes with
  an ADR in `docs/decisions/`.
