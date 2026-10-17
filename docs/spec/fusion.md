# Fusion

Fusion (Autodesk Fusion 360) is a read and drive target for the user
parameters of a design. The contract the adapter meets is adapters.md,
the numbers it moves are ir.md. This document states what is specific
to Fusion. Emit is not provided (adapters.md 6 matrix): push covers it.

## 1. Shape

One Python package, `adapters/fusion/` (`cadetta-fusion`), on the
`cadetta.adapter` base (bindings.md 3.4), plus a small add-in that runs
inside Fusion.

| Part | Runs in | Contents |
|---|---|---|
| `cadetta_fusion` | the cadetta-spawned process | protocol (`adapter.py`), read (`read.py`), drive (`drive.py`), the `FusionSession` seam and its JSON-RPC client (`session.py`) |
| `addin/CadettaFusion` | Fusion | serves the Fusion API over a local socket |

`initialize` reports `capabilities = ["read", "drive"]`,
`native_units = "mm"`, `precision = 6`, platforms windows and macos.

## 2. Mapping: user parameters

Only Fusion user parameters are bound; they are the one Fusion
construct that is a named, typed, expression-bearing value in the
document and is what an engineer expects to edit from outside. Model
parameters (sketch dimensions) are out of 0.1.0: they are generated
names (`d12`) that renumber, which adapters.md 3.1 forbids as a stable
locator.

| Fusion | Binding |
|---|---|
| user parameter `NAME` | locator kind `global`, `locator = "NAME"`, `native_unit` = the parameter's unit |

A parameter's `expression` is what Fusion stores (`10 mm`,
`width * 2`); its `unit` is the unit string Fusion reports. The
internal Fusion unit (centimetres, radians) never reaches cadetta: the
add-in reports the expression and unit as authored, and cadetta works
from those.

## 3. Units

- Read: a literal expression (`12.5 mm`, `3 in`, `45 deg`) is exact:
  its magnitude and unit become the value (ir.md); `unit_scale` maps the
  unit to a dimension and a rational factor. A bare number takes the
  parameter's reported `unit`; a unitless parameter is dimension 1.
- A parameter whose expression refers to other parameters or functions
  (`width * 2`) is `exact: false`, carries the expression in `expr`, and
  emits Q006, because Fusion evaluates it (adapters.md 4).
- A unit that is not recognised is not read: R002 warning, naming the
  parameter.
- Push: the value is written as `<magnitude> <unit>` using the unit given
  with the value, else the parameter's current unit. Magnitudes are
  written at full precision (no float noise: via `repr`).

## 4. Read

`read` takes `files` (open, read, close what it opened, never save) or
an open `session`. All designs read produce one `<pkg>::Observed`
component, `<pkg>` the lowercased stem of the first file, with imported
provenance (`adapter: fusion`, `file`, `native_name`) and one `global`
binding per parameter (adapters.md 7). Names that are not identifiers
are mangled and uniquified per adapters.md 3.1.

## 5. Drive and atomic push

`open` attaches to an already-open document (`attached`, never closed
by the adapter) or opens it. A dirty or read-only document is refused
with R009 before any push (adapters.md 2.2).

`push` validates every change first, then applies the valid ones in one
`doc.apply` call that the add-in executes as a unit:

1. Locator absent from the design: R003, status `skipped`.
2. Value not a literal number with optional unit, or a ref target that
   is not a user parameter: R010, status `rejected`.
3. Expression equal to the current one (whitespace-insensitive): status
   `skipped`, no write.
4. The existing expression is not a literal: it is overwritten and R006
   warns.
5. `mode = ref`: the expression becomes the target parameter's name, so
   Fusion keeps the link.
6. If the single apply fails (bad expression, Fusion refusal), the
   add-in restores every parameter it touched and replies with an error:
   the adapter reports R004, every attempted locator `failed`, nothing
   changed, and `save` skips the file.

`regen` re-reads each design's parameters: Fusion recomputes on every
write, so a clean listing means the design still evaluates. `save` writes
modified files whose push succeeded and returns their blake3 digests.
`close` closes only documents the session opened.

## 6. Transport

Fusion's Python interpreter exists only inside Fusion, so the adapter
process cannot import `adsk`. The add-in therefore listens on
`127.0.0.1:47360` (`CADETTA_FUSION_ADDR` overrides it on both sides) and
speaks newline-delimited JSON-RPC 2.0, one request in flight. The adapter
connects lazily on the first tool-touching call; if the add-in is not
running, `initialize` still succeeds and later calls return an
internal error saying how to start it.

| Method | Params | Result |
|---|---|---|
| `fusion.version` | - | version string |
| `doc.open` | `path` | `{path, dirty, readonly, attached}` |
| `doc.parameters` | `path` | list of `{name, expression, unit, value, comment}` |
| `doc.apply` | `path`, `edits: [{name, expression}]` | null; atomic |
| `doc.save` | `path` | null |
| `doc.close` | `path` | null; closes without saving |

The Fusion API is main-thread only; the add-in hands each request to
the main thread with a custom event and waits for the reply.

## 7. Design rationale

- Add-in plus socket, not a script launched per call: a running Fusion
  holds the user's documents, and attaching must see them (adapters.md
  2.2); launching Fusion per call is also far too slow.
- Local TCP, not stdio: Fusion owns the add-in process, so the adapter
  cannot spawn it and has no pipes to it. Binding only loopback keeps it
  unreachable from the network.
- A `FusionSession` Protocol seam (like FreeCAD's backend): the whole
  protocol is tested against a fake that records a transcript, so CI
  needs no Fusion.
- `unit_scale` is copied from the FreeCAD adapter rather than shared, to
  keep the adapters independent until a third consumer justifies moving
  it into `cadetta.adapter`.

## 8. Verification status

- CI (`.github/workflows/fusion.yml`): ruff and the fake-session
  transcript tests in `adapters/fusion/tests/unit` (adapter protocol,
  read, push, atomic failure and rollback, R003/R004/R006/R009/R010, the
  JSON-RPC client over a real socket).
- Live run attempted 2026-10-17 on nova (Windows 11 ARM64): Fusion
  launches but stops at its sign-in screen (no Autodesk account is signed
  in), so the add-in never loaded; evidence in
  `adapters/fusion/evidence/dw3nemp-live/`. No document was opened.
- Still not done: a live run against a signed-in Fusion. The
  add-in in `addin/CadettaFusion` is written against the documented
  `adsk` API and has not been executed in Fusion. A live run must open,
  read, push, save and close a document, logging open documents before
  and after, and leave none open.
