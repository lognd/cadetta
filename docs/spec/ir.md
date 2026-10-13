# The intermediate representation

The IR is the one data model every lint and every adapter speaks. It is
tool-agnostic: nothing in it is specific to a CAD product except the
opaque native locators held in bindings.

## 1. Two flavours, one schema

| Flavour | Produced by | Contains |
|---|---|---|
| Design IR | the compiler, from `.cdt` source | parameters with expressions, dependencies, provenance, fits, checks, waivers |
| Observed IR | an adapter `read`, from CAD files | parameters with literal values, native names and locators; no expressions |

Both serialize to the same JSON schema, `cadetta.ir/1`. Lints that need
only values run on either flavour; lints that need structure (D, F, C)
run on design IR; drift lints (R) compare the two through bindings.

Observed IR is what makes adoption incremental: a team with existing
SOLIDWORKS parts can `cadetta import` them, lint the observed IR
immediately, and then scaffold `.cdt` source from it.

## 2. Canonical paths

```
path      = namespace "::" Component [ "/" part ] "." param
example     acme.rover::Axle.end_diameter
            acme.rover::Rover/front_left.hub_bore
            acme.rover::wheel_axle.shaft          (fit members)
```

A canonical path is the identity of a parameter across source, IR,
bindings, waivers and diagnostics. Paths are stable under formatting and
under reordering; they change only by rename, which `was` records.

## 3. Entities

```
Project    { schema: "cadetta.ir/1", flavour, namespace, components[],
             fits[], checks[], waivers[], digest }
Component  { path, class: ots|model|assembly, doc?, source?, params[],
             parts[], span? }
Param      { path, kind, dimension, value: Value, expr?, deps[],
             provenance, source?, was[], attrs, doc?, span?, digest }
Value      { nominal: Rational, lower?: Rational, upper?: Rational,
             exact: bool, approx?: f64, authored_unit }
Provenance = authored | derived | imported { adapter, file, native_name }
Fit        { path, hole_class, shaft_class, on, nominal, fit_type, hole, shaft }
Check      { path, lhs, cmp, rhs, verdict?: holds|violates|undecided }
Waiver     { code, target, because, expires?, target_digest }
Binding    { path, adapter, locator, native_unit }
```

- `Rational` serializes as a string `"num/den"` in SI base units, with
  `den > 0` and the fraction reduced. JSON floats never carry a
  magnitude.
- `span` is `file:line:col-line:col`, present in design IR.
- `deps` lists canonical paths the expression reads. The dependency graph
  is acyclic (D003 otherwise).

## 4. Digests

Every Param and Component has a digest: BLAKE3 over its canonical
encoding (path, kind, value, expr text normalized by the formatter,
source, attrs). The digest excludes spans and doc comments.

Digests make three things cheap and exact: waiver staleness (a waiver
binds `target_digest`; W002 when it no longer matches), drift
attribution (R001 names which side moved since the last sync), and
incremental adapter pushes (only changed digests are pushed).

## 5. Stability

`cadetta.ir/1` is versioned. Adding optional fields is a minor change;
anything else bumps the schema version. The JSON Schema is generated
from the Rust types and committed under `docs/schemas/`.
