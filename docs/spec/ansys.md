# Ansys

Ansys is a read and drive target for the input parameters of a
Workbench project. The contract the adapter meets is adapters.md, the
numbers it moves are ir.md. This document states what is specific to
Ansys and records the product choice. Emit is not provided
(adapters.md 6 matrix): push covers it. Status: specification only; no
code exists and no Ansys install has been available (section 7).

## 1. Product choice

Ansys is a suite, not one API. Three entry points could carry a
binding:

| Option | API | What a binding would be |
|---|---|---|
| Workbench parameters (PyAnsys) | `ansys-workbench`/PyWorkbench scripting, journals | named project parameters (`P1`, `P2`, ...) with a unit and an expression, plus the project update |
| Mechanical | `ansys-mechanical-core` (embedded or remote Python) | named selections, geometry and mesh settings, load and material values inside one analysis |
| SpaceClaim | SpaceClaim scripting (IronPython) | CAD dimensions of the geometry model |

Decision: Workbench parameters, driven through PyAnsys.

Rationale:

- Parameters are the construct Ansys itself exposes as the interface of
  a project: a named, typed, unit-bearing value that crosses
  application boundaries (geometry, mesh, solver, post). That is the
  same role as a Fusion user parameter (fusion.md 2) and a FreeCAD
  spreadsheet alias, and adapters.md 3.1 requires a stable locator.
  Mechanical and SpaceClaim have no equivalent across-the-suite name;
  their dimensions are generated (`D1`, `Sketch1.D2`) and renumber.
- The update cycle is what makes Ansys distinct: a parameter change is
  only meaningful after an update, which regenerates geometry, remeshes
  and may solve. Only the Workbench level owns that cycle.
- PyAnsys is the supported, versioned Python route and works with the
  remote and the local install, which fits the external-adapter model
  (adapters.md 5) with a Python package like FreeCAD and Fusion.
- Mechanical and SpaceClaim stay reachable through Workbench: a
  Mechanical or geometry value only needs to be promoted to a Workbench
  parameter (the engineer does this once, in Workbench) to become
  bindable. This is the documented way to expose them, so no second
  adapter is needed in 0.1.0.

Not chosen, and why: Mechanical scripting would bind to analysis
internals that change per analysis type and has no stable names;
SpaceClaim scripting is geometry-only and Windows-only, so it adds
neither results nor the update cycle.

## 2. Shape

One Python package, `adapters/ansys/` (`cadetta-ansys`), on the
`cadetta.adapter` base (bindings.md 3.4). It uses the same session-seam
design as Fusion (fusion.md 7): an `AnsysSession` Protocol, a real
implementation over PyAnsys, and a fake that records a transcript for
CI.

`initialize` reports `capabilities = ["read", "drive"]`,
`native_units = "mm"`, `precision = 6`, platforms windows and linux
(wherever Ansys is installed).

## 3. Mapping: Workbench parameters

Only input parameters are bound. Output parameters are results of the
solve, not design inputs, and are read-only observations.

| Workbench | Binding |
|---|---|
| input parameter `Pn` with a user-given name | locator kind `global`, `locator = "<name>"`, `native_unit` = the parameter's unit |
| input parameter with no user name | locator `"Pn"` (the ID), which is stable within a project |
| output parameter | not bound; reported by `regen` only (section 6) |

The parameter name (display name) is preferred over the `Pn` ID because
it is what the engineer edits; the ID is the fallback. Two parameters
with the same name are uniquified per adapters.md 3.1 with the ID
suffix. Names that are not identifiers are mangled per the same rule.

## 4. Units

- A parameter is a value plus a unit string (`mm`, `MPa`, `deg`, `C`).
  A literal value is exact: magnitude and unit become the value
  (ir.md); `unit_scale` maps the unit to a dimension and rational
  factor, as in the FreeCAD and Fusion adapters.
- A parameter driven by an expression or by another parameter is
  `exact: false`, carries the expression in `expr`, and emits Q006,
  because Ansys evaluates it (adapters.md 4).
- Ansys temperature units are offset units (`C`, `F`); they are mapped
  as absolute kelvin differences only when the binding is a delta.
  A binding on an offset unit is read as an absolute value and pushed in the
  parameter's own unit, never converted by a scale alone.
- An unrecognised unit is not read: R002 warning naming the parameter.
- Push writes `<magnitude> <unit>` using the unit given with the value,
  else the parameter's current unit, at full precision (via `repr`).

## 5. Read

`read` takes `files` (`.wbpj` project archives) or an open `session`.
Each project produces one `<pkg>::Observed` component, `<pkg>` the
lowercased stem of the first file, with imported provenance
(`adapter: ansys`, `file`, `native_name`) and one `global` binding per
input parameter (adapters.md 7). Reading opens the project without
updating it, reads the parameter set, and closes what it opened
without saving.

## 6. Drive, update and atomic push

`open` attaches to an already-open project (`attached`, never closed by
the adapter) or opens it. A project that is dirty or read-only is
refused with R009 before any push (adapters.md 2.2).

`push` validates every change first, then applies the valid ones to the
parameter set in one scripted step:

1. Locator absent: R003, status `skipped`.
2. Value not a literal number with optional unit, or a ref target that
   is not an input parameter: R010, status `rejected`.
3. Equal to the current value: status `skipped`, no write.
4. The existing value is an expression: it is overwritten and R006
   warns.
5. `mode = ref`: the parameter becomes an expression of the target
   parameter so Ansys keeps the link.
6. If the step fails, every parameter touched is restored from the
   values read at the start of the push; the adapter reports R004, every
   attempted locator `failed`, and `save` skips the file.

Push writes parameters but does not update the project: an update can
take minutes to hours and consume a solver license, so it is separate.

`regen` is the update cycle. It runs the project update (geometry,
mesh, solve for the systems that are out of date), then reads the
output parameters and returns them as observations in the regen
result. It reports R004 if the update fails and leaves parameters as
pushed (they are inputs the engineer can inspect). `regen` takes an
optional `timeout`; exceeding it is reported as R004 with the elapsed
time and the project is left for the engineer to stop. `save` writes
modified projects whose push succeeded and returns their blake3
digests. `close` closes only projects the session opened.

## 7. Licensing and test host

- Every operation that opens a project needs an Ansys license, and
  `regen` needs a solver license for the systems it solves. Licenses are
  site-specific (a license server, a student seat, or a trial), so the
  adapter reads no license configuration itself: it relies on the
  standard Ansys environment (`ANSYSLMD_LICENSE_FILE`) and surfaces a
  license failure as an internal error carrying the Ansys message.
- Ansys is not redistributable and not on GitHub-hosted runners, so no
  CI job can run it. There is no free tier suitable for CI.
- On 2026-10-17 no Ansys install was found on nova (the Windows goway
  host); nothing in this spec has been run against Ansys. A live run
  needs a host with Ansys Workbench and a license; until one exists,
  this adapter is specification only and is gated on that host.
- What CI can cover once the code exists: the fake-session transcript
  tests (protocol, read, push, atomic failure and restore,
  R003/R004/R006/R009/R010), exactly as for Fusion (fusion.md 8).

## 8. Design rationale

- Workbench level, not Mechanical or SpaceClaim: section 1.
- Push separated from update: updates are expensive and license
  consuming; engineers want to batch parameter changes and then update
  once, and `regen` already means "recompute and check" in
  adapters.md.
- Session seam rather than calling PyAnsys directly, so the entire
  protocol is testable without an install.
- Offset temperature units handled explicitly (section 4) because a
  scale-only conversion silently gives the wrong value for `C` and `F`.

## 9. Verification status

Not implemented and not run. Open items before 0.1.0 code: a host with
Ansys and a license (section 7), confirmation of the PyAnsys call that
sets a parameter expression, and the exact `.wbpj` open and close
behaviour with respect to the dirty flag.
