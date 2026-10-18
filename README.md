<p align="center">
  <img src="https://raw.githubusercontent.com/lognd/cadetta/main/docs/assets/cadetta-banner.svg" alt="cadetta: typed, toleranced design parameters, linted once and emitted to every CAD tool." width="100%"/>
</p>

<p align="center">
  <a href="https://github.com/lognd/cadetta/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://github.com/lognd/cadetta/actions/workflows/ci.yml"><img src="https://github.com/lognd/cadetta/actions/workflows/ci.yml/badge.svg" alt="CI status"></a>
  <img src="https://img.shields.io/badge/status-0.1.0-orange.svg" alt="Status: 0.1.0">
</p>

# Cadetta

Cadetta treats the numbers that define a mechanical design as source
code. You write every dimension once, as a quantity with units, a
tolerance and a citation. Cadetta checks that the design still works
for every value in tolerance and drives your CAD files from that one
source, so nobody retypes a number. It also tells you exactly what a
change invalidates.

Status: **0.1.0 released** (Rust core and `cadetta` CLI, Python binding,
C ABI, FreeCAD and SOLIDWORKS adapters, editor support); the specification
is **draft 1.5**. The spec is in
[docs/spec/](https://github.com/lognd/cadetta/blob/main/docs/spec/README.md).

---

## Why

A CAD model is good at geometry and bad at knowing why a number is what
it is. On a real project:

- **The same value lives in many places.** A bearing's bore drives a
  shaft, a housing, a spacer and a drawing note. Each file keeps its own
  copy, and when the vendor changes the part, someone has to find them
  all.
- **Numbers lose their meaning.** `12.012mm` might be a press fit, a
  typo or a measurement. The tolerance, the fit class and the datasheet
  page it came from are in someone's head or a spreadsheet, not in the
  model.
- **Tolerance thinking is done by hand.** Will the bolt still go through
  when both hole patterns are at their limits? Does the stack-up still
  clear? Engineers work it out on paper, once, and it silently goes out
  of date.
- **Shop knowledge is retyped.** Every screw means looking up the
  clearance hole, the tap drill for that material, the engagement
  length and the screw length.
- **Changes have unknown reach.** Swapping a supplier, a material or a
  motor ripples through checks, sign-offs, analysis scripts and CAD
  files, and nothing lists which.
- **Moving values into CAD is manual.** Open, retype or re-import,
  rebuild, check, repeat. A dimension edited directly in the CAD tool
  quietly diverges from everything else.

Software solved the same problems with types, compilers, linters, build
systems and CI. Cadetta brings those tools to design parameters, and
the CAD tool becomes a build target.

## How it works

Cadetta runs one cycle every time you check or sync a design:

1. **Write.** The design's numbers live in `.cdt` files: plain text in
   git, reviewed like code. Each value carries its units, tolerance and
   source.
2. **Compile.** Cadetta evaluates the source into an intermediate
   representation (IR). The IR is tool-agnostic: every lint, query and
   adapter reads it, and so can anything that reads JSON.
3. **Check.** The linter and the checks run over the IR for every value
   in tolerance and every approved alternate. The obligation graph marks
   which sign-offs, solver results and CAD files are now stale.
4. **Sync.** Adapters, the only part of cadetta that touches a CAD tool,
   compare the design with what the CAD file holds and with what was
   last synced. They push the values that changed, rebuild the model,
   and read the result back.
5. **Reconcile.** Anything that differs on the CAD side becomes drift
   you can see. You resolve it by keeping the design or by writing the
   CAD-side value back into the source. Existing CAD files enter the
   cycle the same way, through `import`.

The CAD tool keeps owning shapes and mates. Cadetta owns the numbers,
the reasons for them, and the checks on them.

## A short tour

```text
// src/lib.cdt of package acme.rover

use std::fasteners::ScrewJoint
use std::materials::Material

cite vendor = "https://vendor.example/4471-B.pdf"

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

ots Colson2Series: Caster @"Colson 2 Series catalog, p.14" {
  hub_bore    = 12mm H8
  hub_width   = 7/8in
  load_rating = 540N
  unit_cost   = 13.90USD
}

model Axle(wheel: Caster, span: Length) {
  fit seat     = p6 on wheel.hub_bore     // the shaft side, from ISO 286
  end_diameter = seat.shaft
  length       = 2 * wheel.hub_width + span
}

model Frame {
  span     = 300mm
  material = Material::Al6061T6
  datum top  = origin.offset(z = 40mm)
  datum site = top.offset(x = 60mm)
  tapped: [Point2 on site; 4] = rect_pattern(2, 2, 30mm, 20mm) [+-0.05mm]
}

model MotorPlate {
  thickness = 5mm
  datum base = origin
  holes: [Point2 on base; 4] = rect_pattern(2, 2, 30mm, 20mm) [+-0.1mm]
}

assembly Rover {
  part wheel[4]: Caster = Vendor4471B alt [Colson2Series]
  part frame: Frame
  part axle[2]: Axle(wheel = wheel[0], span = frame.span)
  part plate: MotorPlate
  place frame.origin at origin
  place plate.base at frame.site
  part plate_screws: ScrewJoint(thread = M4, clamp = plate.thickness, host = frame,
                                at = plate.holes, onto = frame.tapped)
  check load_ok for w in wheel: w.load_rating * 4 >= 1500N
  check axle_fits: axle[0].length <= frame.span + 40mm
}
```

From that, cadetta knows:

- **The axle end diameter** is the p6 shaft for the wheel's actual bore.
  If the wheel changes, so does the shaft. No number was copied.
- **The motor plate's four screws:**
  - clearance holes, the tap drill for 6061, thread and drill depth,
    screw length and the drawing callout are all computed and cited;
  - both hole patterns line up at every tolerance limit;
  - the screws are in the bill of materials.
- **`load_ok` and `axle_fits`** hold for the primary wheel, and are
  checked again for each approved alternate.
- **The wheel's end-of-life date** is tracked: after June 2027, using it
  without a viable alternate is an error.

When something does not hold, the message says why, where and for which
variant:

```text
warning[I002]: approved alternate is not viable
  --> cad/rover.cdt:59:3
   |
59 |   check axle_fits: axle[0].length <= frame.span + 40mm
   |   ^^^^^^^^^^^^^^^ violated under wheel=Colson2Series
   |
   = note: axle[0].length = 344.45mm (2 * 7/8in + 300mm); limit 340mm
   = note: holds for the primary: 338.1mm
   = help: the end-of-life wheel has no viable alternate after 2027-06 (L001)
   = path: acme.rover::Rover.axle_fits
```

The swap you were planning for the end-of-life wheel would not have
fit, and you learn that before anyone orders a part. The same check
failing for the primary would be `error[C001]` and would fail the
build.

## What it can do

**A language for design intent.**
- Quantities are first-class: `5in`, `3/4in`, `12mm H7`,
  `10mm [+-0.05mm]`. Values are exact rationals, so converting inches to
  millimetres and back loses nothing.
- Units and kinds are checked: a torque is never assigned to an energy.
- References replace copies, `let` holds stack-ups, and tables hold
  catalog data.
- Every value can cite its source: a document, a page, `@measured` or
  `@assumed`.

**Tolerance-aware checks.**
- `check` and `require` are three-valued: holds, violates, or undecided
  over the tolerance range.
- Stack-ups are exact affine forms, so a dimension that appears twice is
  counted once.
- ISO 286 fits, including one-sided fits such as `H7 on motor.shaft`.

**Positions without a geometry kernel.**
- Points are typed by datum the way lengths are typed by unit.
- Parts are placed in a tree, so a second bolted connection is a check
  rather than a solve.
- Hole patterns, alignment, coaxiality and bolt clearance float are
  checked under tolerance.

**Shop knowledge as a library.**
- `ScrewJoint` covers clearance holes, tap drills by material,
  engagement, screw length, counterbores and callouts.
- Fits, materials and retaining rings are included.
- The numbers a shop disagrees about live in an overridable, cited
  policy. Metric is in 0.1.0; inch and inserts follow in 0.2.0.

**Swappable parts.**
- Designs depend on interfaces, not SKUs.
- Every approved alternate is evaluated continuously.
- `impact --swap` previews a swap before you make it: the values and
  verdicts that change, the CAD files to re-sync, the sign-offs
  invalidated, and the cost, lead-time and mass deltas. `swap` applies
  it.

**An obligation graph.**
- Every check, fit, sign-off, citation, solver result and CAD file is
  pinned to Merkle digests of exactly what it read.
- `impact --since HEAD` lists what your change invalidates before you
  commit.
- `why --stale` names the upstream edit that made a sign-off stale.
- Role-based sign-offs live in a mergeable `cadetta.lock`.

**Packages.**
- Shared shop policies and parts libraries are packages, depended on
  by version; a product line is a workspace.
- Modules come from the file tree, and a library's API is its `pub`
  items (`pub use` re-exports a curated surface).
- Dependencies are pinned in `cadetta.lock` by digest, and
  `cadetta update` re-resolves them. A library update shows up in the
  obligation graph as exactly the checks and sign-offs it makes stale.

**External analysis.**
- Values from a Python or MATLAB script come in through a small JSON
  protocol.
- Results are cached by input digest, checked for kind, and marked stale
  when any input changes.

**CAD integration.**
- SOLIDWORKS is the primary target: read, push, rebuild and read back
  through COM.
- `sync` is a three-way merge. An edit made in SOLIDWORKS is reported,
  never silently overwritten, and `import --update` writes it back into
  the source.
- `import` adopts an existing part.
- A zero-install path writes a SOLIDWORKS linked-equations file.
- FreeCAD is the CI-testable reference adapter; JSON and CSV emitters
  are built in.

**Python and everything else.**
- `import cadetta` gives exact quantities and every CLI query from
  Python, plus decorators for writing solvers and adapters.
- Any other language uses the JSON outputs and protocols.

## How it fits the engineering flow

| Stage | You do | Cadetta does |
|---|---|---|
| Adopt | `cadetta import solidworks Bracket.SLDPRT`, review the proposal, `--accept` | Turns globals, equations and dimensions into named, bound `.cdt` source |
| Design | Edit `.cdt`; reference, don't copy; declare fits, joints and checks | Computes derived values; checks every value in tolerance and every alternate |
| Build | `cadetta sync` | Pushes changed values into SOLIDWORKS, rebuilds, reads back, and reports drift and rebuild errors |
| Analyse | Write a solver in Python or MATLAB | Runs it, caches the result, and makes dependent checks stale when inputs change |
| Review | `cadetta ack check:**/seat* --role tolerance_review` | Records sign-off against the exact digests reviewed |
| Change | `cadetta impact --since HEAD` before committing | Lists changed values, flipped verdicts, stale sign-offs and CAD files to re-sync |
| Re-source | `cadetta impact --swap acme.rover::Rover/wheel acme.rover::Colson2Series`, then `swap` | Previews and applies the swap, with commercial deltas |
| Update | `cadetta update acme.shop`, then `cadetta impact --worktree` | Re-resolves a dependency, rewrites the lock, and lists the verdicts and sign-offs it flips |
| Release | `cadetta check --release` in CI | Fails on anything violated, undecided past policy, stale or unsigned |

## What it is not

Cadetta is not a CAD tool and not a geometry kernel. It does not model
solids, solve mates, find interference between shapes, or do GD&T
tolerance zones. It is also not a PDM system. It owns the numbers, the
reasons for them and the checks on them, and leaves geometry to the CAD
tool.

## Install

The core is Rust. The `cadetta` command ships on crates.io and on PyPI;
the PyPI package is also importable (`import cadetta`):

```sh
cargo install cadetta      # or
pip install cadetta        # or: uv tool install cadetta
```

Adapters are separate: `cadetta-freecad` ships on PyPI; the SOLIDWORKS
adapter (C#, `adapters/solidworks`) and the Fusion adapter
(`adapters/fusion`) are built from this repository. A C ABI
(`libcadetta_capi`, `cadetta.h`) ships as release archives for C, C++,
MATLAB and Julia (see [bindings/README.md](bindings/README.md)).

## Quickstart

```sh
cadetta init --adapter solidworks          # cadetta.toml, src/lib.cdt
cadetta import solidworks Bracket.SLDPRT   # propose names and bindings
cadetta import --accept                    # write bindings and .cdt
cadetta check                              # compile, lint, check
cadetta update                             # re-resolve dependencies, write the lock
cadetta sync --dry-run                     # what a sync would change
cadetta sync                               # push, rebuild, read back
cadetta why acme.rover::Rover/axle[0].end_diameter
```

## Editor support

Syntax highlighting and `cadetta lsp` (diagnostics, formatting, hover, code lens, go to definition) for VS Code, Sublime Text, Vim/Neovim, Emacs and JetBrains: see [editors/README.md](editors/README.md).

## Documentation

| Document | Contents |
|---|---|
| [Specification](https://github.com/lognd/cadetta/blob/main/docs/spec/README.md) | Index, scope, open questions |
| [Quantities](https://github.com/lognd/cadetta/blob/main/docs/spec/quantities.md) | Units, kinds, tolerances, exactness |
| [Language](https://github.com/lognd/cadetta/blob/main/docs/spec/language.md) | `.cdt` syntax and semantics |
| [Positions and datums](https://github.com/lognd/cadetta/blob/main/docs/spec/geometry.md) | Points, placements, alignment |
| [Obligations](https://github.com/lognd/cadetta/blob/main/docs/spec/obligations.md) | Digests, sign-offs, impact and swap queries, the lock file |
| [Adapters](https://github.com/lognd/cadetta/blob/main/docs/spec/adapters.md) | Sync, import, the adapter protocol, CAD targets |
| [Standard library](https://github.com/lognd/cadetta/blob/main/docs/spec/stdlib.md) | Fits, materials, fasteners, `ScrewJoint` |
| [Lints](https://github.com/lognd/cadetta/blob/main/docs/spec/lints.md) | Every diagnostic code |
| [Config and CLI](https://github.com/lognd/cadetta/blob/main/docs/spec/config-cli.md) | `cadetta.toml` and every command |
| [Bindings](https://github.com/lognd/cadetta/blob/main/docs/spec/bindings.md) | Python and the JSON catch-all |
| [Decisions](https://github.com/lognd/cadetta/tree/main/docs/decisions) | Why things are the way they are |
| [Golden corpus](https://github.com/lognd/cadetta/tree/main/tests/golden) | Realistic designs with hand-evaluated expected results |
| [History](https://github.com/lognd/cadetta/tree/main/docs/history) | How the spec evolved |

## Project layout

| Path | Contents |
|---|---|
| `docs/spec/` | The specification |
| `docs/decisions/` | Architecture decision records |
| `docs/schemas/` | Generated JSON schemas of the IR and the CLI, solver and adapter documents |
| `tests/golden/` | Golden corpus: realistic designs with hand-evaluated expected results |
| `docs/history/` | How the spec evolved: pilot rounds and review reports |
| `crates/` | The Rust workspace: units, syntax, sema, fits, geometry, ir, lint, obligations, solve, variants, adapter, log, and the `cadetta` binary (`cadetta-cli`) |
| `std/` | The standard library package (`std::fits`, `std::materials`, `std::fasteners`, `std::inserts`, ...), embedded in the binary |
| `bindings/` | Language bindings, one directory per language (`bindings/python` is the importable `cadetta`, `bindings/c` the C ABI); other languages use the JSON interfaces |
| `adapters/` | External CAD adapters: `freecad` (the reference adapter), `solidworks` (C#, the primary target), `fusion` |
| `editors/` | Syntax highlighting and `cadetta lsp` setup for VS Code, Sublime Text, Vim/Neovim, Emacs, JetBrains, tree-sitter |
| `design/` | The grimble architecture model |
| `tickets/` | The frob ticket ledger (do not edit by hand) |

## Contributing and license

See [CONTRIBUTING.md](https://github.com/lognd/cadetta/blob/main/CONTRIBUTING.md)
and [SECURITY.md](https://github.com/lognd/cadetta/blob/main/SECURITY.md).
MIT licensed; see [LICENSE](https://github.com/lognd/cadetta/blob/main/LICENSE).
