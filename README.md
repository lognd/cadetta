<p align="center">
  <img src="https://raw.githubusercontent.com/lognd/cadetta/main/docs/assets/cadetta-banner.svg" alt="cadetta: typed, toleranced design parameters, linted once and emitted to every CAD tool." width="100%"/>
</p>

<p align="center">
  <a href="https://github.com/lognd/cadetta/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://github.com/lognd/cadetta/actions/workflows/ci.yml"><img src="https://github.com/lognd/cadetta/actions/workflows/ci.yml/badge.svg" alt="CI status"></a>
  <img src="https://img.shields.io/badge/status-spec%20draft%201.3-orange.svg" alt="Status: spec draft 1.3">
</p>

# Cadetta

Cadetta is the single source of truth for the parameters of a CAD design.
You write every dimension once, as a typed quantity with its units, its
tolerance and where it came from. Cadetta lints those parameters and
moves them into and out of your CAD tools, so nobody retypes a number.

It starts with mechanical CAD (SOLIDWORKS, FreeCAD, Onshape, Fusion,
Inventor) and is designed to extend to other kinds of CAD, such as
electrical design in KiCad.

Status: specification draft. See [docs/spec/](https://github.com/lognd/cadetta/blob/main/docs/spec/README.md).

---

## Why

Most CAD tools have parametric features, but their parameters are flat,
labeled numbers. That breaks down in three ways.

- **Duplication.** One dimension of an off-the-shelf part drives
  dimensions in several of your parts, and each part keeps its own copy.
  Nothing says which copy is the truth.
- **Naming.** Every hole and cylinder gets a parameter called "Diameter",
  so people prefix names by hand, inconsistently. Working out what a
  parameter means takes effort.
- **Magic numbers.** Fits need tolerances, so engineers type a nominal
  and a tolerance by hand. A later reader cannot tell whether `12.012mm`
  is a mistake or a press fit.

On top of that, getting values into the CAD tool is manual work: open,
retype or re-import, rebuild, check.

## How

- **Quantities are first-class.** `5in`, `3/4in`, `12mm H7` and
  `10mm [+-0.05mm]` are values with a dimension, a kind and a tolerance.
  A bare number is an error. Values are stored exactly, so converting
  inches to millimetres and back loses nothing.
- **References, not copies.** A model parameter is an expression over
  the parameters it depends on. Values you can derive must not be
  retyped, and the linter flags values that look copied.
- **Canonical names.** Every parameter has one path, such as
  `acme.rover::Axle.end_diameter`. Adapters map each CAD tool's native
  names onto these paths.
- **Fits and checks are declarations.** `fit seat = p6 on wheel.hub_bore`
  computes the shaft from the ISO 286 tables; `check` and `require`
  state what must hold, evaluated over every value in tolerance.
- **Interfaces make parts swappable.** A design depends on what it needs
  from a part, not on a vendor SKU. Approved alternates are checked
  continuously, and `cadetta impact --swap` previews a swap for cost,
  lead time, end of life or performance before you make it.
- **Provenance and obligations.** Every value can cite where it came
  from (`@vendor:"p.2"`, `@measured`). The obligation graph tracks which
  checks, sign-offs, solver results and CAD files a change invalidates.
- **A standard library of shop knowledge.** Declare a screw joint with a
  thread, the clamped thickness and the material it threads into, and
  the clearance hole, tap drill, thread depth, screw length and callout
  are computed and cited. Shop rules are data you can override.
- **External solvers.** Values from a Python or MATLAB analysis come in
  through a small JSON protocol, cached and checked for staleness.
- **A tool-agnostic IR.** Source compiles to an intermediate
  representation that every lint and every adapter reads. Adapters can
  `emit` parameter files, `read` existing CAD files, and `drive` a live
  CAD session: push values, rebuild, read back and check for drift.

## Example

A `.cdt` file:

```text
namespace acme.rover

cite vendor = "https://vendor.example/4471-B.pdf"
cite colson = "https://colson.example/2-series.pdf"

/// What the rover needs from a caster wheel, whoever makes it.
interface Caster {
  hub_bore: Length
  hub_width: Length
  load_rating >= 400N
}

#[lifecycle(eol, "2027-06")]
ots Vendor4471B: Caster @vendor {
  hub_bore    = 12mm H7               @vendor:"p.2"
  hub_width   = 3/4in
  load_rating = 450N
  unit_cost   = 11.20USD
}

ots Colson2Series: Caster @colson {
  hub_bore    = 12mm H8
  hub_width   = 7/8in
  load_rating = 540N
  unit_cost   = 13.90USD
}

model Axle(wheel: Caster, span: Length) {
  fit seat     = p6 on wheel.hub_bore     // shaft side of the fit
  end_diameter = seat.shaft
  length       = 2 * wheel.hub_width + span
}

model Frame {
  span = 300mm
}

assembly Rover {
  part wheel[4]: Caster = Vendor4471B alt [Colson2Series]
  part frame: Frame
  part axle[2]: Axle(wheel = wheel[0], span = frame.span)
}
```

## Install

The core is Rust. The `cadetta` command ships on crates.io and on PyPI;
the PyPI package is also importable (`import cadetta`) for Python
interop:

```sh
cargo install cadetta      # or
pip install cadetta        # or: uv tool install cadetta
```

The FreeCAD adapter is the Python package `cadetta-freecad`. Releases
are placeholders until 0.1.0.

## Quickstart (planned)

```sh
cadetta init                                # cadetta.toml and a hello-world design
cadetta check                               # compile and lint
cadetta import freecad part.FCStd           # adopt an existing CAD file
cadetta sync                                # push, rebuild, read back, check for drift
cadetta impact --swap wheel Colson2Series   # preview a part swap
cadetta why Axle.end_diameter               # where a value comes from
```

## Project layout

| Path | Contents |
|---|---|
| `docs/spec/` | The specification: quantities, language, IR, obligations, solvers, adapters, lints, config and CLI |
| `pilots/` | Syntax pilots: realistic designs written against the spec, with friction reports |
| `docs/decisions/` | Architecture decision records |
| `crates/` | The Rust workspace: units, syntax, sema, fits, ir, lint, obligations, solve, adapter, and the `cadetta` binary |
| `adapters/freecad/` | The FreeCAD reference adapter (Python, JSON-RPC on stdio) |
| `bindings/` | Language bindings over the Rust core, one directory per language (`bindings/python`: the importable `cadetta` package); any other language uses the JSON interfaces |
| `design/` | The grimble architecture model |
| `tickets/` | The frob ticket ledger (do not edit by hand) |

## Contributing and license

See [CONTRIBUTING.md](https://github.com/lognd/cadetta/blob/main/CONTRIBUTING.md)
and [SECURITY.md](https://github.com/lognd/cadetta/blob/main/SECURITY.md).
MIT licensed; see [LICENSE](https://github.com/lognd/cadetta/blob/main/LICENSE).
