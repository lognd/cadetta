# Language bindings

Cadetta's core is Rust. Other languages reach it in one of two ways,
and every capability is reachable the first way.

## 1. Tiers

1. **JSON, the catch-all.** Any language that can run a process and
   read JSON uses cadetta today:
   - the CLI with `--format json` (config-cli.md), whose outputs have
     schemas in `docs/schemas/`, one per command, listed in the
     config-cli.md 3 command table (the single list), plus the
     diagnostic record `cadetta.diag/1` (lints.md 1.1) that several of
     them embed;
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
crates, section 4) is the base for C, C++, MATLAB (MEX) and Julia.

## 2. Repository layout

`bindings/<language>/` holds one binding: its native crate (a workspace
member), its package files, tests, and README. Bindings need their
language's toolchain, so the workspace's default members and the Rust
gates exclude them. Each binding has its own CI job, and each publishes
to its language's registry at the workspace version (lockstep).

| Binding | Directory | Registry | Status |
|---|---|---|---|
| Python | `bindings/python` | PyPI `cadetta` | 0.1.0 |
| C ABI | `bindings/c` | release archives | 0.1.0 (run/version/free) |

## 3. Python

`pip install cadetta` installs the `cadetta` command (the Rust CLI,
through a console script) and the importable package. The package is a
PyO3 extension (`cadetta._core`) and a Python layer of pydantic models.

Error contract. Operators on `Quantity` raise `cadetta.QuantityError`
(it carries `code`, the Q-code the compiler would report): a dimension
or kind violation is a programmer bug, as in any Python numeric
library. Fallible calls at an I/O, parse or query boundary return typani
`Result`; `.unwrap()` suits scripts. `Err` means I/O, configuration or
usage only (the CLI's exit code 2); a design with error diagnostics is
`Ok(report)` (the CLI's exit code 1), and the caller reads
`report.diagnostics`.

### 3.1 Reading a design

```python
import cadetta

project = cadetta.Project.open("path/to/project").unwrap()
design = project.evaluate().unwrap()        # the design IR (ir.md)

m = design.get("acme.rover::Rover/axle[0].end_diameter").unwrap()   # Member
d = m.value                                  # Quantity
d.nominal.to_exact("mm")   # Fraction(12, 1): exact, or None when inexact
d.max.to_float("mm")       # upper limit after tolerance, as a float
d.mid.to_float("in")       # a float only when asked for
m.kind, m.citation         # "param" (the IR member kind), its CiteRef (ir.md 3)
design.citation(m.citation.cite).unwrap().fields["rev"]   # Result[Citation, PathNotFound]
d.kind                     # "Length" (the quantity kind)
```

A variant read, on the fixture package
`bindings/python/tests/fixtures/alt-wheel` (`part wheel: Caster =
Vendor4471B alt [Colson2Series]` and `bore: Length = wheel.hub_bore` in
`Rover`, language.md 3.2):

```python
bore = design.get("acme.rover::Rover.bore").unwrap()
bore.value.nominal.to_exact("mm")   # Fraction(12, 1): the primary, Vendor4471B
alt = bore.variant("acme.rover::Rover/wheel", "acme.rover::Colson2Series")
alt.value.nominal.to_exact("mm")    # Fraction(15, 1): bore under the alternate
bore.variant("acme.rover::Rover/wheel", "acme.rover::Other")   # None: not an alternate
```

- `design.get(path) -> Result[Member, PathNotFound]`; `design[path]` is
  not offered. Paths are instance paths: a parameterized component such
  as `Axle(wheel, span)` exists only as `part axle[2]`, so its members
  are `Rover/axle[0].end_diameter` and `[1]`, never `Axle.end_diameter`
  (ir.md 2, language.md 3.1).
- `Member` wraps the generated pydantic model of the JSON member (kept
  as `.model`, field for field); `.value` is the one field rebuilt as a
  Rust-backed `Quantity` (`Quantity._from_ir`, from the IR `Value`
  alone), and `.kind` is the IR member kind, as the CLI prints it. The
  quantity kind is `.value.kind`: the unique built-in kind with the
  value's dimension, else anonymous (for example a temperature, which
  is ambiguous between `Temperature` and `TemperatureDelta`).
  `Value` carries `dim`, the exponent vector over the SI base
  dimensions plus angle and currency, so a `Quantity` is built from the
  IR alone; `kind` names the kind (`"Length"`).
- `Quantity` mirrors quantities.md: an exact magnitude, dimension, kind
  and optional tolerance. `.nominal .min .max .mid .mmc .lmc` are
  computed in the Rust core, never re-implemented in Python, and return
  untoleranced Quantities. `to_exact(unit) -> Fraction | None` is
  `None` when the value is inexact; `to_float(unit) -> float` always
  works. There is no `.to`, so the return type is never a union.
- `member.variant(slot, alternate) -> Member | None` reads the value
  under a variant (language.md 3.2); `slot` is the canonical
  `Type/part` path and `alternate` the qualified component name, the
  same strings as `impact --swap SLOT ALT`. It reads the IR member's
  `variants` entry keyed `<slot>=<alternate>` (ir.md 2), so a member that
  does not depend on the slot, or whose value is unchanged, has no
  entry and returns `None`.
- Arithmetic follows the language's kind rules and carries tolerance
  (affine forms, quantities.md 5.4), so Python results agree with
  checks. A violation raises `QuantityError`, whose `.code` is the
  compiler's Q-code. `q.as_kind("Length")` mirrors `as` (quantities.md)
  and raises `QuantityError` (Q002, the same code `as` gives in the
  compiler) when the dimension differs.
- Decided in implementation: `to_exact` is also `None` when a factor
  of pi remains after dividing by the unit (`180deg` in `rad`), since a
  `Fraction` cannot hold it. `.mmc`/`.lmc` need a role: a plain parsed
  quantity raises `QuantityError` (E010) until `q.with_role("hole" |
  "shaft")`. Affine temperature units (`degC`, `degF`) and ISO fit
  classes are not accepted by `Quantity` in 0.1. Only integer scalars
  mix with a Quantity in operators; floats never enter a stored value.
- `cadetta.quantity("12mm [+-0.1mm]")` parses the source syntax and
  returns `Result[Quantity, QuantityError]`. A parse failure, including
  a split literal such as `"12 mm"` (Q007), is `Err(QuantityError)`
  with `.code` set; it is never `Ok` with a diagnostic, and the CLI
  exit-code mapping is `Err` = 2.
- Error types of operators. `QuantityError.code` is a Q-code for
  dimension and kind violations and parse failures, and may also carry
  an E-code where the compiler reports the same condition under a lint
  code (`.mmc`/`.lmc` without a role: E010). A non-integer operand
  (`q * 1.5`) is not a quantity error: `Quantity` returns
  `NotImplemented` and Python raises `TypeError`, with no Q-code.
- Interop: `.to_pint(registry)` with the optional `cadetta[pint]`
  extra (implemented as `cadetta.pint_interop.to_pint(q, registry)`,
  returning a `Result`; SI coherent units, nominal only, tolerance
  dropped, currency dimensions rejected); floats for numpy via `to_float(unit)`.

### 3.2 Surface

`Project.open` reads `cadetta.toml` with the config-cli.md 1.6 walk:
from `path` (default the working directory), or from the directory of
`config`, up to the nearest `cadetta.toml` with a `[package]` or
`[workspace]` table; none is `Err(ProjectError)`. Opening inside a
workspace member scopes to that member; at the workspace root `package`
is `None`, `modules` is `[]` and `evaluate()` returns the CLI's usage
error. `project.modules` is computed from the `.cdt` files under the
package `src` (`lib.cdt` is the package root, implicit parent
directories are listed), because the IR `modules[]` is not yet populated
by the CLI. `evaluate()` runs the `ir` verb and validates its JSON with
the generated models in `cadetta.models.ir` (regenerate with
`datamodel-codegen` from `docs/schemas/cadetta.ir.1.json`).

Every query and mutation verb has a method except `init`, `cite pin`,
`lock verify`, `ir --observed`, `frames`, `display`, `lsp`, `ir-validate`
and `lint FILE`, which stay CLI-only (they edit repository files around
the project, read no design, or serve an editor). `names --observe` is
not offered either. Each returns `Result[Model, CadettaError]`
where the model is generated from the verb's JSON schema (section 1),
so a binding returns exactly what the CLI's JSON returns.

| Python | CLI | Model (schema) |
|---|---|---|
| `Project.open(path=None, config=None)` | root discovery of a package or workspace, `--config` (config-cli.md) | `Project` |
| `project.package` | the `[package]` name and version | `Package` |
| `project.modules` | the module paths under `src`, per package | `list[str]` |
| `project.update(name=None)` | `update [NAME]` | `update.json` |
| `project.evaluate()` | `ir` (design) | design IR (`cadetta.ir/1`) |
| `project.check(release=False, as_of=None, module=None)` | `check` | `check.json` |
| `project.why(path)` | `why` | `why.json` |
| `project.impact(path)` | `impact PATH` | `impact.json` |
| `project.impact_since(since=None)` | `impact --since REV` (`--worktree` when `since` is `None`) | `impact-since.json` |
| `project.why_stale(obligation, role=None)` | `why --stale` | `why.json` |
| `project.impact_swap(slot, alternate)` | `impact --swap SLOT ALT` | `swap.json` |
| `project.swap(slot, alternate, reason, as_=None)` | `swap` | `swap.json` |
| `project.compare(interface)` | `compare` | `compare.json` |
| `project.obligations(stale=False, role=None, release=False, as_of=None)` | `obligations` | `obligations.json` |
| `project.solve(names=None, variants=False)` | `solve` | `SolveReport` (hand-written; text parsed) |
| `project.emit(adapters=None)` | `emit` | `emit.json` |
| `project.sync(adapters=None, dry_run=False, only=None, force_design=False, no_regen=False, module=None)` | `sync` | `sync.json` |
| `project.import_(adapter, files, accept=False, update=False, write=False)` | `import` | `import.json` |
| `project.ack(selector, role, reason, baseline=False, release=False, dry_run=False, as_=None)` | `ack` | `ack.json` |
| `project.explain(code)` | `explain` | `explain.json` |
| `project.cite_list(format="json", module=None)` | `cite list` | `cite-list.json` |
| `project.names(adapter=None, module=None)` | `names` | `names.json` |
| `project.bind(path, adapter, locator, kind, **fields)` / `project.unbind(path, adapter, locator)` | `bind` / `bind --remove` | `bind.json` |
| `cadetta.units(kind=None)` | `units` | `units.json` |
| `cadetta.format_source(text)` | `fmt` | `str` |

Decided in implementation (mutation and IO verbs): `update`, `emit`,
`import_` and `ack` return the generated `Update`, `EmitReport`,
`ImportReport` and `AckOutput` models; `import_(accept=True)` runs
`import --accept` (the adapter and files are ignored). `sync` returns the
generated `SyncReport` (`sync.json`: records, diagnostics, regen, emitted,
manual, lock_written) and a payload failing the schema is `Err`. `solve`
still returns a hand-written `SolveReport` (`solved`, `fresh`, `text`) parsed
from the CLI's text lines `solved PATH` and `fresh PATH`, although the CLI
now has `solve-report.json` (`cadetta.solve-report/1`); moving to a generated
model is not done yet. The code is `cadetta.project_ops`.

`slot` is a canonical `Type/part` path (`acme.rover::Rover/wheel`) and
`alternate` a component name, qualified when it lives in another
module or package (`acme.shop::Colson2Series`), exactly as on the command
line. `Project` is a snapshot: it reads sources at `open`, and
`project.reload()` re-reads them. `evaluate()` returns the primary
evaluation; variants are read through `member.variant`.

### 3.3 Writing solvers

```python
from cadetta.solve import solver

@solver(inputs={"load": "Force", "span": "Length", "width": "Length",
                "allowable": "Pressure"},
        outputs={"wall": "Length"}, significant=6)
def wall_sizing(load, span, width, allowable):
    # t = sqrt(6 M / (sigma b)), M = load * span / 4: Length after sqrt
    return {"wall": ((6 * load * span / 4) / (allowable * width)) ** 0.5}

if __name__ == "__main__":
    wall_sizing.serve()       # speaks cadetta.solve/1 on stdin/stdout
```

The decorator reads the request, converts inputs to Quantities (a
toleranced input arrives as a Quantity with its tolerance), checks the
declared kinds both ways, and writes the response with the solver's
version. Outputs are written as decimal strings rounded to `significant`
digits (default 6, overridable per output), never `repr(float)`, so
the consumer sees no M001 or Q006 noise (solvers.md 3). A wrong kind is
a protocol error naming the output, not a silently wrong number.

### 3.4 Writing adapters

`cadetta.adapter.Adapter` implements the JSON-RPC loop of adapters.md 5:
`initialize`, `emit`, `read`, `push`, `regen`, `shutdown` (and
`open`, `save`, `close` for drive adapters) are methods to override.
Observed IR is built with the same models; `unit_scale`, `safe_name`,
`package_name`, `observed_member`, `observed_binding` and
`observed_project` in the same module are the shared read-side helpers. Logs go to stderr only,
because stdout carries the protocol. The FreeCAD and SOLIDWORKS
adapters are built on it.

### 3.5 What Python does not do

Python does not define parameters that feed back into the design
outside the solver protocol: `.cdt` source stays the single source of
truth. Programs may generate `.cdt` text and format it with
`cadetta.format_source`, but evaluated values flow only from source.

## 4. C ABI

`bindings/c` builds `libcadetta_capi` and `include/cadetta.h`. The
surface is `cadetta_version`, `cadetta_run(argc, argv, &out, &err)` and
`cadetta_free`. `cadetta_run` runs the CLI entry point in-process and
returns its exit code with captured stdout and stderr, so with
`--format json` it returns the CLI's document by construction. Decision
(2026-10-16): one entry point rather than a function per query, because
the JSON documents and their schemas are already the stable contract
(tier 1); a typed per-query C API would duplicate them and drift. Exit
0 and 1 carry a document; 2 is usage or I/O; negative returns are ABI
failures (bad argv, core panic), never an abort. Its CI job replays the
conformance `check`/`config`/`eval` fixtures through a C program and the
CLI and compares exit code and stdout.

## Open questions

- OQ-16 (closed 2026-10-13, bindings.md 3.1): Python `Quantity`
  arithmetic carries tolerance (affine forms, quantities.md 5.4), so
  Python results agree with checks.
