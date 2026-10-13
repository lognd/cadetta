# The intermediate representation

The IR is the one data model every lint, query and adapter speaks. It is
tool-agnostic: nothing in it is specific to a CAD product except the
opaque native locators held in bindings.

## 1. Two flavours, one schema

| Flavour | Produced by | Contains |
|---|---|---|
| Design IR | the compiler, from `.cdt` source | everything: values, expressions, dependencies, citations, interfaces, parts and alternates, fits, checks, requires, solvers, waivers |
| Observed IR | an adapter `read`, from CAD files or inspection data | parameters with values, native names and locators; expressions when the native format exposes them (SOLIDWORKS equations, FreeCAD spreadsheet formulas) |

Both serialize to `cadetta.ir/1` JSON. Value lints (D001, M001, N, Q)
run on either flavour; structural lints (D002, F, C, I, L) on design IR;
drift lints (R) compare the two through bindings.

Observed IR makes adoption incremental: `cadetta import` reads existing
CAD files, lints them at once, and scaffolds `.cdt` source.

## 2. Canonical paths

```
path      = namespace "::" Type [ "/" part [ "[" index "]" ] ]* "." member
example     acme.rover::Axle.end_diameter
            acme.rover::Rover/wheel[2].hub_bore
            acme.rover::Axle.seat.shaft            (fit members)
```

A canonical path is the identity of a node across source, IR, bindings,
waivers, the lock file and diagnostics. Paths survive formatting and
reordering; they change only by rename or move, which `was` records.

## 3. Entities

```
Project    { schema, flavour, namespace, components[], interfaces[], tables[],
             solvers[], checks[], requires[], waivers[], citations[], digest }
Component  { path, class: ots|model|assembly|process, params_sig?, implements[],
             citation?, attrs, members[], parts[], fits[], span?, digest }
Member     { path, kind, analysis: bool, value?: Value, bound?: Bound, expr?,
             deps[], role?, citation?, was[], attrs, provenance, doc?, span?,
             variants?: { "<slot>=<alternate>": Value }, digest, closure }
Value      { nominal: Exact | Inexact, angle: int, tol?: Affine, conservative: bool,
             designed?: Exact, authored: string }
Exact      { r: "num/den", pi: int }                 // magnitude = r * pi^pi, SI base
Inexact    { f64, root: path }                       // root inexact operation
Affine     { terms: [ { var: path, lo: Exact, hi: Exact } ] }
Bound      { op: "<=" | ">=" | "in", lo?: Exact, hi?: Exact }
Part       { path, type, count?, primary, alternates[], args }
Fit        { path, hole, shaft, clearance: Value, type, required?: Bound }
Datum      { path, parent: path, ops: [ offset{x,y,z: Value} | rotate{axis, angle: Value}
             | flip{axis} ], exact: bool }               // geometry.md 2
Point      { path, datum: path, coords: [Value] }        // each coord also a Member
Placement  { part: path, datum: path, on: path, ops[], digest, closure }
Check      { path, expr, verdicts: { primary, per_alternate{"<slot>=<alt>"} },
             conservative, waived_for[] }
Solver     { path, script, inputs{}, outputs{}, deps[], result_digest?, stale }
Citation   { name, target, locator?, kind: url|file|standard|measured|assumed|estimate,
             file_digest? }
Provenance = authored | derived | imported { adapter, file, native_name }
Binding    { path, adapter, locators: [ { locator, kind: global|dimension|property,
             mode: value|ref, native_unit } ] }
```

- Magnitudes are never JSON floats: `Exact` is a reduced fraction string
  plus a pi exponent. `Inexact.f64` is the only float, and names its root.
- `designed` holds the value before process compensation; `nominal` is
  the drawn value (language.md 3.8).
- `deps` are canonical paths; the graph is acyclic (D003).
- `analysis` marks a `let` (quantities.md 5.3): never emitted or bound.
- `variants` holds, per part slot and alternate, only the values that
  differ from the primary evaluation (variant evaluation, language.md
  3.2). Keys are `"Rover/wheel=Colson2Series"`.

## 4. Digests

Every node has two digests, both BLAKE3:

- `digest` (own): over the node's canonical encoding (path, kind, value,
  normalized expression text, citation, attrs, and for parts the
  primary, alternates and arguments). It changes when the node's own
  text does.
- `closure` (Merkle): over the own digest plus the closure digests of
  its `deps`, sorted by path. It changes when anything the node reads,
  transitively, changes, including a cited file's digest and a solver
  result's digest.

Granularity is the member of a component definition; an instance path
(`Rover/wheel[2].hub_bore`) has the closure of its member plus the
digests of only those part arguments the member reads, transitively.
For an interface-typed slot the instance closure also includes the
part node's own digest (primary, alternates, arguments), so a swap
makes every instance path of the slot stale, and nothing else: a check
reading only another part's arguments keeps its closure. Spans and doc comments are excluded, so
formatting never changes a digest. Obligations and waivers bind closure
digests (obligations.md 2, W002); drift attribution (R001) and
incremental pushes use own digests of emitted values.

## 5. Stability

`cadetta.ir/1` is versioned. Adding optional fields is minor; anything
else bumps the schema. JSON Schemas are generated from the Rust types
and committed under `docs/schemas/`.
