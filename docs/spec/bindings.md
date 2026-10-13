# Language bindings

Cadetta's core is Rust. Other languages reach it in one of two ways,
and every capability is reachable the first way.

## 1. Tiers

1. **JSON, the catch-all.** Any language that can run a process and
   read JSON uses cadetta today:
   - the CLI with `--format json` (config-cli.md), whose outputs have
     schemas in `docs/schemas/`;
   - the IR itself, `cadetta.ir/1` (ir.md);
   - the solver protocol `cadetta.solve/1` (solvers.md), for computing
     values;
   - the adapter protocol, JSON-RPC 2.0 on stdio (adapters.md 5), for
     reaching a CAD tool.
2. **Native bindings**, one per language, for interop inside a process:
   values as native objects, no subprocess, no parsing. A binding is a
   thin wrapper over the Rust crates and adds no semantics. For every
   query it offers, it returns exactly what the CLI's JSON returns, and
   its tests replay the conformance corpus to show it.

A C ABI (`cadetta-capi`, a stable `extern "C"` surface over the same
crates) is the planned base for C, C++, MATLAB (MEX) and Julia. It is
built when the first of those is needed, not before.

## 2. Repository layout

`bindings/<language>/` holds one binding: its native crate (a workspace
member), its package files, tests, and README. Bindings need their
language's toolchain, so the workspace's default members and the Rust
gates exclude them. Each binding has its own CI job, and each publishes
to its language's registry at the workspace version (lockstep).

| Binding | Directory | Registry | Status |
|---|---|---|---|
| Python | `bindings/python` | PyPI `cadetta` | 0.1.0 |
| C ABI | `bindings/c` | release archives | later |

## 3. Python

`pip install cadetta` installs the `cadetta` command (the Rust CLI,
through a console script) and the importable package. The package is a
PyO3 extension (`cadetta._core`) and a Python layer of pydantic models.
Fallible calls return typani `Result`; `.unwrap()` suits scripts.

### 3.1 Reading a design

```python
import cadetta

project = cadetta.Project.open("path/to/project").unwrap()
design = project.evaluate().unwrap()        # the design IR (ir.md)

d = design["acme.rover::Axle.end_diameter"]
d.nominal.to("mm")       # Fraction(12, 1): exact, never a float by surprise
d.max.to("mm")           # upper limit after tolerance
float(d.mid.to("in"))    # a float only when asked for
d.kind, d.citation       # "Length", the source
```

- `Quantity` mirrors quantities.md: an exact magnitude (`Fraction`
  times a power of pi), dimension, kind, angle exponent and optional
  tolerance. `.to(unit)` is exact; inexact values say so
  (`.exact == False`). Arithmetic between Quantities follows the
  language's kind rules; a violation is a `Result` error carrying the
  same Q-code the compiler would report.
- `cadetta.quantity("12mm [+-0.1mm]")` parses the source syntax.
- Interop: `.to_pint(registry)` with the optional `cadetta[pint]`
  extra; floats for numpy via `.to(unit)` then `float()`.

### 3.2 Queries

`project.check()`, `project.impact(...)`, `project.impact_swap(slot,
alternate)`, `project.compare(interface)` and `project.why(path)`
return pydantic models generated from the same JSON schemas as the CLI
(`impact.json`, `compare.json`, `why.json`, the diagnostic schema). Diagnostics
carry their code, severity, path, span and message.

### 3.3 Writing solvers

```python
from cadetta.solve import solver

@solver(inputs={"load": "Force", "span": "Length"}, outputs={"wall": "Length"})
def wall_sizing(load, span):
    return {"wall": (load * span / allowable).to_kind("Length")}

if __name__ == "__main__":
    wall_sizing.serve()       # speaks cadetta.solve/1 on stdin/stdout
```

The decorator reads the request, converts inputs to Quantities, checks
the declared kinds both ways, and writes the response with the
solver's version. A wrong kind is a protocol error naming the output,
not a silently wrong number.

### 3.4 Writing adapters

`cadetta.adapter.Adapter` implements the JSON-RPC loop of adapters.md 5:
`initialize`, `emit`, `read`, `push`, `regen`, `shutdown` are methods
to override. Observed IR is built with the same models. Logs go to
stderr only, because stdout carries the protocol. The FreeCAD and
SOLIDWORKS adapters are built on it.

### 3.5 What Python does not do

Python does not define parameters that feed back into the design
outside the solver protocol: `.cdt` source stays the single source of
truth. Programs may generate `.cdt` text and format it with
`cadetta.format_source`, but evaluated values flow only from source.

## Open questions

- OQ-16: Should `Quantity` arithmetic in Python carry tolerance (affine
  forms, quantities.md 5.4) or nominal values only? Recommendation:
  carry it; the core already computes it, so dropping it would make
  Python results disagree with checks.
