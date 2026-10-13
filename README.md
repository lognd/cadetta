# Cadetta

Cadetta is the single source of truth for the parameters of a CAD design.
You write every dimension once, as a typed quantity with its units, its
tolerance and where it came from. Cadetta lints those parameters and
moves them into and out of your CAD tools, so nobody retypes a number.

It starts with mechanical CAD (SOLIDWORKS, FreeCAD, Onshape, Fusion,
Inventor) and is designed to extend to other kinds of CAD, such as
electrical design in KiCad.

Status: specification draft. See [docs/spec/](docs/spec/README.md).

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

- **Quantities are first-class.** `5in`, `12mm H7` and `10mm +- 0.05mm`
  are values with a dimension, a kind and a tolerance. A bare number is
  an error. Values are stored exactly, so converting inches to
  millimetres and back loses nothing.
- **References, not copies.** A model parameter is an expression over
  the parameters it depends on. Values you can derive must not be
  retyped, and the linter flags values that look copied.
- **Canonical names.** Every parameter has one path, such as
  `acme.rover::Axle.end_diameter`. Adapters map each CAD tool's native
  names onto these paths.
- **Fits are declarations.** `fit wheel_axle = H7/p6 on Wheel.hub_bore`
  computes both sides of the fit from the ISO 286 tables.
- **A tool-agnostic IR.** Source compiles to an intermediate
  representation that every lint and every adapter reads. Adapters can
  `emit` parameter files, `read` existing CAD files, and `drive` a live
  CAD session: push values, rebuild, read back and check for drift.

## Example

```text
namespace acme.rover

/// 5 inch caster wheel, vendor part 4471-B.
ots Wheel {
  source "https://vendor.example/4471-B.pdf#page=2"
  diameter = 5in            // alternatively 5"
  hub_bore = 12mm H7
  hub_width = 0.75"
}

fit wheel_axle = H7/p6 on Wheel.hub_bore

model Axle {
  end_diameter = wheel_axle.shaft
  length = 2 * Wheel.hub_width + Frame.span
}

model Frame {
  span = 300mm
}

assembly Rover {
  part front_left: Wheel
  part axle: Axle
  part frame: Frame
}
```

## Quickstart (planned)

```sh
cadetta init                      # cadetta.toml and a hello-world design
cadetta check                     # compile and lint
cadetta import freecad part.FCStd # adopt an existing CAD file
cadetta sync                      # push, rebuild, read back, check for drift
```

## Project layout

| Path | Contents |
|---|---|
| `docs/spec/` | The specification: quantities, language, IR, adapters, lints, config and CLI |
| `docs/decisions/` | Architecture decision records |
| `design/` | The grimble architecture model |
| `tickets/` | The frob ticket ledger (do not edit by hand) |
