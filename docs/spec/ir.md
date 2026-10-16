# The intermediate representation

The IR is the one data model every lint, query and adapter speaks. It is
tool-agnostic: nothing in it is specific to a CAD product except the
opaque native locators held in bindings.

## 1. Two flavours, one schema

| Flavour | Produced by | Contains |
|---|---|---|
| Design IR | the compiler, from `.cdt` source | everything: values, expressions, dependencies, citations, interfaces, parts and alternates, fits, checks, requires, solvers, waivers |
| Observed IR | an adapter `read`, from CAD files or inspection data | parameters with values, native names and locators; expressions when the native format exposes them (SOLIDWORKS equations, FreeCAD spreadsheet formulas); native frames and component instances with their transforms |

Both serialize to `cadetta.ir/1` JSON. Value lints (D001, M001, N, Q)
run on either flavour; structural lints (D002, F, C, I, L) on design IR;
drift lints (R) compare the two through bindings.

Observed IR places every read parameter as a `param` member of one
synthetic component `<package>::Observed`, named by the native name made
path-safe (non-alphanumerics become `_`, collisions get `_2`, `_3`); the
member carries `provenance = imported` with the true native name, the
native expression in `expr` when exposed (`deps` stays empty: native
expression syntax is the adapter's), and one binding with the locator.
Citations, parts, fits, checks and solvers are empty. Two lists carry
what geometry verification needs (geometry.md 6, adapters.md 3.4):
`frames[]`, one `{ name, document, transform }` per native frame read
(a SOLIDWORKS coordinate system), and `instances[]`, one `{ component,
document, reference?, transform }` per component instance of an
assembly, where `reference` is the instance's free-text reference
property when the tool has one. A transform is a 3x4 matrix of `Value`s
in the document's frame (rotation columns, then translation in the
adapter's native unit), read like any native number: a binary64 that is
not a short decimal is `exact: false` with its reason.

Observed IR makes adoption incremental: `cadetta import` reads existing
CAD files, lints them at once, and scaffolds `.cdt` source.

## 2. Canonical paths

```
path      = PKG { "::" module } "::" Type [ "/" part [ "[" index "]" ] ]* "." member
member    = IDENT { "." IDENT | "[" index "]" }
index     = INTEGER | "*"                    // "*" only in bindings (below)
PKG       = IDENT { "." IDENT }              // the dotted package name; module = snake_case IDENT
example     acme.rover::Axle.end_diameter
            acme.rover::Rover/wheel[2].hub_bore
            acme.rover::Axle.seat.shaft            (fit members)
            acme.rover::Bracket.holes[1].x         (index on a member)
            acme.rover::Bracket.top.offset.x       (datum offset)
            acme.rover::Bracket.top.rotate.z       (datum rotation)
            acme.rover::Rover/bracket.place.offset.x     (placement, first offset, axis x)
            acme.rover::Rover/bracket.place.offset[1].x  (second offset)
            acme.rover::Rover/feet[2].place.offset.x     (array placement, instance 2)
            acme.rover::refs::sunon25              (a cite item; see below)
            acme.rover::drive::motor::Gm42A.rated_torque   (item in module drive::motor)
```

A canonical path is the identity of a node across source, IR, bindings,
waivers, the lock file and diagnostics. A variant is named `SLOT=ALT`:
the slot's instance path and the alternate's component path
(`acme.rover::Rover/wheel=acme.rover::Colson2Series`). Inside source,
and in diagnostics about the current module, the package and module
prefix and the assembly may be omitted wherever they are unambiguous
(`wheel=Colson2Series`); IR keys and lock records always use the full
form.

- Variant names (`cadetta-ir` `variant_name.rs`): the full form splits
  at the single `=` into a slot instance path (must carry a `/slot`
  step) and a bare component path. A short alternate (`Colson2Series`,
  or module-qualified `b::Colson`) resolves against the components
  visible in the current module: exactly one match by item name (and
  module suffix) is required, none is unknown, several is ambiguous
  and must be qualified.

An evaluated value has the path of the instance it belongs to
(`acme.rover::Rover/axle[0].end_diameter`); a definition path
(`acme.rover::Axle.end_diameter`) names the declaration, which is what
obligations of definition level (obligations.md 2.1) and waivers bind.
A parameter-free component read by type name is its one implicit
instance (language.md 3.1), so its definition path is also its value
path. Under a process, `PATH.drawn` names the drawn value (language.md
3.8). Paths survive formatting and
reordering; they change only by rename or move, which `was` records.

- Parsing (`cadetta-ir` `path.rs`): an index is an integer or `*`; the
  item name starts uppercase; only the full form
  parses, and sema expands abbreviations first. Reading a
  parameter-dependent member by a definition path (no `/` step) is E011.
- A cite item's path is `PKG { "::" module } "::" name`: its last
  segment is the snake_case item name, so it is read as an item path
  only where the schema says so (`Citation.path`, `CiteRef.cite`,
  `attest citation:` records), never inferred from its shape.
- Datum offsets are `Type.<datum>.offset.x|y|z` and rotations
  `.rotate.x|y|z`, the n-th op of a kind (n > 0) indexed
  (`.offset[1].x`); placement op arguments use the same scheme under
  `Assembly/part.place` (`Assembly/part.place.offset.x`,
  `Assembly/feet[2].place.rotate.z` for instance 2 of an array placement), one per op
  axis (geometry.md 2, 4). The placement itself is the node
  `Assembly/part.place`, which an `instance` locator binds. Each is a
  Member with provenance `authored`, so bindings, R001, waivers and
  `why` attach to it.
- Pattern generators (`rect_pattern`, `circle_pattern`) are values with
  named members (`holes.pitch_x`, `holes.pitch_y`, `holes.radius`,
  `holes.start`, `holes.rotation`, `holes.count_x`, `holes.count_y`,
  `holes.count`) that bind; the derived coordinates are analysis-only
  unless the points are hand-listed.
- Parameterized components exist only as instances (language.md 3.1):
  their parameter-dependent members have instance paths only. A binding
  may name one instance or all of them with
  `Rover/axle[*].end_diameter`; the instances share arguments, so the
  value is the same up to independent tolerance variables. A `[*]`
  binding emits once, at the emit point of instance 0. Because the
  instances share arguments, no source makes another instance's
  emitted value differ; R012 is reserved for that case (adapters.md 3).
- Part-array instance members: a counted part `part axle[2]: Axle` whose
  callee has no parameters lowers one member per instance and per
  parameter or `let` of the callee (`Rover/axle[0].len`,
  `Rover/axle[1].len`; indexes are 0-based). Each carries the callee
  definition's value and depends on the definition member
  (`Axle.len`), with derived provenance. An array of a parameterized
  callee (`part arm[2]: Arm(len = 10mm)`) expands to one instance per
  index, each with its own arguments: its members lower as
  `Rover/arm[0].reach`, `Rover/arm[1].reach` with the instance's value.
- Instance vs definition level: an obligation targets a definition path
  when its closure reads no component parameter or interface-typed slot,
  and an instance path otherwise; instances with identical argument
  closures form one obligation (obligations.md 1).

## 3. Entities

```
Project    { schema, flavour, package, modules[], dependencies[], components[],
             interfaces[], tables[], solvers[], checks[], requires[], waivers[],
             citations[], frames[], instances[], digest }
Component  { path, class: ots|model|assembly|process, params_sig?, implements[],
             citation?: CiteRef, attrs, members[], parts[], fits[], datums[], points[],
             placements[], span?, digest }
Member     { path, kind, analysis: bool, priv: bool, value?: Value, bound?: Bound, expr?,
             deps[], role?, citation?: CiteRef, was[], attrs, provenance, doc?, span?,
             variants?: { "<slot>=<alternate>": Value }, digest, closure }
Value      { nominal: Exact | Inexact, dim: Dim, angle: int, tol?: Affine,
             conservative: bool, drawn?: Exact, exact: bool, reason?: string,
             unit_assumed?: string, authored: string }
Dim        { m, kg, s, A, K, mol, cd: int, angle: int, currency: { "<ISO>": int } }
Exact      { r: "num/den", pi: int }                 // magnitude = r * pi^pi, SI base
Inexact    { f64, root: path }                       // root inexact operation
Affine     { terms: [ { var: path, lo: Exact, hi: Exact } ] }   // a disc: paired .u/.v terms
Bound      { op: "<=" | ">=" | "in", lo?: Exact, hi?: Exact }
Interface  { path, members[], bounds[], datums[] }
Table      { path, key, cols[], rows[], citation?: CiteRef }
Require    { path, expr, message?, citation?: CiteRef, verdicts: { primary, per_alternate{"<slot>=<alt>"} },
             conservative }
             // a Check without waived_for: a require is never suppressed (obligations.md 4)
Waiver     { code, glob, because, expires?, for_alt?: "<slot>=<alternate>",
             targets_closure }
Process    { path, params[], rules: [ { role, expr } ] }
Part       { path, type, count?, primary, alternates[], args }
             // slot Part.path is "PKG::Type/part" (the `/part` step, no
             // ".member"), per cadetta-sema lower.rs (~137W4EE)
Fit        { path, hole, shaft, clearance: Value, type, required?: Bound }
             // hole/shaft are the REAL operand member paths (~3YZ3NSG), see "Fit operands"
Op         = offset{x,y,z: member ref} | rotate{axis, angle: member ref}
             | flip{axis}                                // shared by Datum and Placement
Datum      { path, parent: path, ops: [Op], variants?, digest, closure }  // geometry.md 2
Point      { path, datum: path, coords: [Value], position?: Exact }   // each coord also a Member; position: zone diameter
Placement  { path, part: path, datum: path, at: path, ops: [Op], variants?, digest, closure }
Check      { path, expr, verdicts: { primary, per_alternate{"<slot>=<alt>"} },
             conservative, waived_for[] }
Solver     { path, script, citation?: CiteRef, inputs{}, outputs{}, deps[], result_digest?, stale }
Citation   { path, type: standard|datasheet|drawing|catalog|book|article|web|quote|document,
             fields{}, file_digest?, digest }                  // language.md 3.9
CiteRef    { cite?: path, builtin?: note|measured|assumed|estimate, text?, locator? }
Provenance = authored | derived | imported { adapter, file, native_name }
Binding    { path, adapter, locators: [ Locator ] }
Locator    { locator, kind: global|dimension|property|frame|instance, mode: value|ref,
             native_unit, ref_to?: path, for?: "<alternate>", config?: string, sign?: 1|-1,
             component?: string, implicit?: global|link|frame|reference,
             tolerance?: drive|ignore }
```

- A disc source variable (geometry.md 3.1, quantities.md 5.4) appears
  as two `Affine` terms whose `var` paths share a stem and end in `.u`
  and `.v`; the pair ranges over the disc `u^2 + v^2 <= 1`, not the box.
  A `[position d]` point lowers one such pair, stem `<point>.position`
  (`Type.holes[i].position.u`), on both its x and y coordinate values.
  Both terms carry the symmetric bounds `+-(|a| + |b|)` of the disc
  coefficients `a`, `b` (for position, `d/2`), a sound outer bound of
  the true radius; each coordinate lists the full pair, so a single
  coordinate's range is looser than its true `[-d/2, d/2]`.
- Observed flavour only: `Project.frames[]` holds `{ name, document,
  transform }` and `Project.instances[]` holds `{ component, document,
  reference?, transform }` (section 1); a transform is a 3x4 array of
  `Value`. Both are empty in design IR.
- Magnitudes are never JSON floats: `Exact` is a reduced fraction string
  plus a pi exponent. `Inexact.f64` is the only float, and names its root.
- `Datum.variants` and `Placement.variants` are keyed like
  `Member.variants`. The composed transform of a datum or placement is
  derived, not stored: a 3x4 matrix of `Exact` (entries from exact
  multiples of 90deg), else `Inexact` with a root; exactness is derived
  from the ops.
- Wire form of `Op` (`cadetta-ir` `geometry.rs`): a one-key object,
  `{"offset": {"x"?: path, "y"?: path, "z"?: path}}`,
  `{"rotate": {"axis": "x|y|z", "angle": path}}` or `{"flip": "x|y"}`;
  arguments are the paths of their Members. A point offset
  (`.offset(p)`) is an `offset` whose Members' expressions read `p`. In
  `Datum.variants` and `Placement.variants` a value is the whole op
  chain for that alternate (`{"<slot>=<alternate>": [Op]}`).
- Citations (language.md 3.9): `Project.citations[]` holds every `cite`
  item the design reaches, sorted by path; `fields` maps field names to
  strings (`author` to a list). A `CiteRef` has exactly one of `cite` (a
  cite item's path) and `builtin`; `text` is a note's text, `locator`
  the string after `:`. `Citation.digest` hashes the type, the fields in
  name order and `file_digest` (the marker `absent` for a missing file);
  a citing node's own digest includes its `CiteRef`, never the cited
  item's digest, so citation drift is S004 alone (section 4).
- `Locator.implicit` records why a binding exists without a bindings
  file entry (adapters.md 3.5): `global` (a native global with the
  path's mangled name, or `bind = "scope"`), `link` (a dimension whose
  equation references such a global), `frame` (a coordinate system
  with a datum's mangled name), `reference` (an instance whose reference
  property names the part). Explicit locators omit it.
- `nominal` is always the designed value (the part as made, what fits,
  checks and reads see). `drawn?` exists only when a process rule
  applied and holds the compensated value that is emitted (language.md
  3.8, quantities.md 5.6). There is no `designed` field.
- `dim` is the exponent vector over the SI base dimensions plus `angle`
  and one exponent per currency; `kind` names the kind (`Length`,
  `Volume/Time`), `dim` makes `.to(unit)` computable from the IR alone.
- `exact: false` with a `reason` marks observed values that the native
  format could not give exactly; `unit_assumed` records a unit supplied
  by `--assume-unit` (adapters.md 3).
- Locator fields: `ref_to` is the native name a `ref`-mode link points
  at; `for` selects a slot binding's alternate (`for = "<alternate>"`,
  R011); `config` is the native configuration; `sign = -1` negates a
  coordinate (geometry.md 6, R007); `component` names the in-assembly
  instance (`wheel<2>`) (emitters/sw-equations.md "SOLIDWORKS naming").
- `Project.package` is `{name, version}`; `modules[]` lists every module
  path of the package, sorted (packages.md 2); `dependencies[]` holds
  `{name, version, source, digest}` per resolved dependency, sorted by
  name (packages.md 1). Items in `src/lib.cdt` sit at the package root.
- Rust model (`cadetta-ir` `model.rs`): `Exact` wraps the units
  `Magnitude`, `Value.dim` is the units `Dim`, digests are `Option`
  until computed, `Project.modules` and `dependencies` are kept sorted
  by their setters, and a member's `Locator.for` is the field
  `for_alternate` in Rust.
- JSON encoding (`cadetta-ir` `json.rs`): `Exact.r` is always
  `"num/den"` (reduced, `den > 0`, integers as `"n/1"`); absent optional
  fields are omitted; enums are lowercase strings (`Sign` is the integer
  `1` or `-1`); `Member.priv`, `Part.type`, `Fit.type` and `Locator.for`
  are the wire names of the Rust fields `private`, `type_name`,
  `fit_type` and `for_alternate`; `Provenance` is `"authored"`,
  `"derived"` or `{ "imported": { adapter, file, native_name } }`. A
  non-finite `Inexact.f64` is a serialization error.
- Item entities (`cadetta-ir` `entities.rs`): `Interface`, `Table` and
  `Process` paths are item paths (`acme.rover::Caster`, no member
  tail). `Interface.members[]` are `{ name, type, optional }`,
  `bounds[]` are `{ name, bound }`, `datums[]` are datum names.
  `Table.key` and `cols[]` are `{ name, type }`; a row is `{ key, cells[] }`
  (no cells for a key-only row) and a cell is a `Value` or
  `{ "variant": name }` for an enum key or column. `Waiver.expires` is
  the ISO date text, `for_alt` the full-form `SLOT=ALT` text, and
  `targets_closure` maps each matched target path to its closure digest
  (empty until reviewed, lints.md 2.1). `Process.params[]` are `Member`s,
  `rules[]` are `{ role, expr }`.
- Obligation and attestation state is not IR: it lives in the
  `cadetta.obligations/1` schema (obligations.md).
- `deps` are canonical paths; the graph is acyclic (D003).
- `analysis` marks a `let` (quantities.md 5.3): never emitted or bound.
- `variants` holds, per part slot and alternate, only the values that
  differ from the primary evaluation (variant evaluation, language.md
  3.2). Keys are the full qualified form
  `"acme.rover::Rover/wheel=acme.rover::Colson2Series"`.
- Lowering (`cadetta-sema` `lower.rs`, `lower::lower`): the lowerer lives
  in sema, not `cadetta-ir`, because sema depends on ir (ir <- sema).
  Rules it fixes:
  - A component path has an empty member tail (`pkg::Axle`).
  - `provenance` is `derived` when the member's `deps` are non-empty,
    else `authored`. A `let` has `kind = let` and `analysis = true`.
  - `nominal` is the designed value; for a process-compensated member
    `drawn` is the compensated nominal (Exact) and there is no separate
    `designed` field.
  - `tol` has one term per source variable the evaluated form depends
    on, with radius `|c|`; the centre-minus-nominal offset is carried by
    the first term (`lo = offset - r`, `hi = offset + r`). A variable
    owned by a member is named by that member's path; an anonymous one is
    named `<member path>.tol<N>`, N being its order within the member.
    A derived parameter keeps its nominal only (no `tol`); a `let` keeps
    the analysis form.
  - `variants` keys are caller-supplied variant names; only values that
    differ from the primary are stored.
  - Datums, points and placements are lowered through `cadetta-geometry`.
    A Datum's `of` and a Placement's `datum` and `at` are definition-level
    paths (`Type.datum`). An array placement is expanded to one Placement
    per instance (`Asm/node[i]`, integer indexes) and skipped, with a
    warning, when its count is unknown. A Point is `Type.p` or
    `Type.holes[i]`; its coordinates are Members `...x|y|z` (kind `let`,
    analysis form, derived from the declaring member) and `position` is the
    zone diameter (`Exact`) when the point is `[position d]`-toleranced,
    otherwise absent.
  - Solvers and waivers lower into `solvers` and `waivers`: a solver keeps
    its script, one input per `in` binding (the expression text), one output
    per `out` (the path of the recorded output member) and the members its
    inputs read as `deps`; its `@cite` lowers to `citation`, documentation
    only and never an input to the solver's digest or closure;
    `result_digest` and `stale` come from the recorded result file. A waiver keeps its glob as written.
  - A `table` lowers to a `Table` (`PKG::Name`) with its key and value
    columns (name and written type), one row per source row (key and one
    cell per column, a quantity `Value` keeping its authored text or an
    enum variant name) and its `@cite` as `citation`; a table that does
    not evaluate, or holds another kind of cell, is skipped.
  - A `require` lowers to a `Require` in `requires[]` (never into `checks[]`; ~X50QNG4): same
    path and folded verdicts (primary and per alternate) as a check, plus its `message` and its
    `@cite` as `citation`. The CLI obligation universe reads `requires[]` as `require` obligations
    (a check-shaped graph node, offered for sign-off only while it holds).
  - Not lowered yet: interfaces
    and bindings; parameterized components lower with valueless
    members. The datum and placement arguments of a parameterized
    instance lower as `datum` / `place` Members of the instantiating
    component (`Box/j.mount.offset.z`), derived, with the per-instance
    value.

## 4. Digests

Every node has two digests, both BLAKE3:

- `digest` (own): over the node's canonical encoding (path, kind, value,
  normalized expression text, `CiteRef` (path and locator), semantic
  attrs, and for parts
  the primary, alternates and arguments). It changes when the node's own
  text does; a change inside the cited source is the citation's own
  digest and S004 (obligations.md 2.3).
- `closure` (Merkle): over the own digest plus the closure digests of
  its `deps`, sorted by path. It changes when anything the node reads,
  transitively, changes, including a solver result's digest.

Granularity is the member of a component definition; an instance path
(`Rover/wheel[2].hub_bore`) has the closure of its member plus the
digests of only those part arguments the member reads, transitively.
For an interface-typed slot the instance closure also includes the
part node's own digest (primary, alternates, arguments), so a swap
makes every instance path of the slot stale, and nothing else: a check
reading only another part's arguments keeps its closure. Spans and doc comments are excluded, so
formatting never changes a digest.

Attributes split in two. Semantic attributes are in the digest:
`process`, `role`, rule roles (`hole`, `boss`, ...), `default`, and the
presence rules `when` and `requires` (language.md 3.11).
Presentational attributes are out of it: `display`, `emit`, `allow` and
the date text of `lifecycle`; changing them never makes an obligation
stale. `emit` is tracked in the sync record instead (adapters.md), so
a changed emit point still re-pushes. Obligations and waivers bind closure
digests (obligations.md 2, W002); drift attribution (R001) and
incremental pushes use own digests of emitted values.

Canonical encoding decisions (`cadetta-ir` `digest.rs`): fields are tagged
and length-prefixed; `Value.authored` (display text) is excluded along with
spans and docs; every attribute except `display`, `emit`, `allow` and
`lifecycle` is semantic; a check's verdicts, `conservative` and `waived_for`
are results and not hashed; a closure hashes the own digest then the dep
closure digests in dep path order; an instance closure hashes the member
closure, the instance path text and, for an interface-typed slot, the slot
part's own digest. Part arguments are nodes keyed `<slot>.<arg>`.

Digest graph additions (`digest.rs`): a component's formal arguments are
nodes keyed `<component>.<param>` so a member reading one resolves; each
fit contributes a fit node plus its `hole` and `shaft` nodes. A commercial
roll-up (`deps.rs`) depends only on parts whose component declares the
rolled member; an optional member such as `unit_cost` is skipped on a part
lacking it instead of becoming a dangling dependency. A read of a bare part
slot depends on the slot's `Type/slot` node; a read of a member of a
dependency-package part depends on that slot too (the foreign component is
not lowered). A dependency on a `fit` whose operands read component arguments
is dropped, since that fit is not derived at the generic level. Such a fit
is instead derived once per instance of the parameterized component
(`fitderive.rs`): `Top/h.seat` for `part h: Housing(bearing = b)`, its
operands read through the instance's argument values; a fit whose argument
is not evaluated for an instance is skipped for that instance.

Fit operands (decision, ~3YZ3NSG): `Fit.hole` and `Fit.shaft` are the
canonical paths of the members the fit constrains, as resolved from the
source and, for a per-instance fit, re-rooted at the instance (`Top/h.seat`
reads `Top/h.bearing.outer_dia`). Rationale: lints (M003, D, unused) and the
obligation graph need the operand, and the field names say "the hole
member"; the alternative (keep synthetic `<fit>.hole` paths and add
`hole_operand`/`shaft_operand`) would leave two meanings per name and make
every consumer choose. The accessor reads `<fit>.hole`, `<fit>.shaft` and
`<fit>.clearance` stay addressable value paths; the digest keys the first
two as nodes under the fit (never under the operand, which has its own
node). A side no member names (the class of a class fit `H7 on d`, both
sides of a class pair) has no operand and keeps the synthetic
`<fit>.hole`/`<fit>.shaft` path. The schema is unchanged (same fields, same
types), only their content.

Fit accessor members: a read of `<fit>.hole`, `<fit>.shaft` or
`<fit>.clearance` is also emitted as a derived `Let` member of the owning
component (provenance `derived`, analysis, `deps` = the fit path), so the
read is an ordinary member with a node. No schema change. The digest keys
`.clearance` on that member node; `.hole` and `.shaft` keep their fit-side
nodes until the `add_fit` follow-up lands.

## 5. Stability

`cadetta.ir/1` is versioned. Adding optional fields is minor; anything
else bumps the schema. JSON Schemas are generated and committed under
`docs/schemas/` (`cadetta.ir.1.json`, `diag.json`).

Decisions (`cadetta-ir` `schema.rs`): the IR types serialize by hand, so
the schema is one definition table (`schema_value`) beside them rather
than a derive; `cadetta.ir.1.json` has `Project` at the root and every
entity of section 3 under `$defs` (including `Dim` and the item entities
`Interface`, `Table`, `Waiver`, `Process`). `Project` carries optional
`interfaces[]`, `tables[]`, `requires[]` and `waivers[]` (minor change; absent means
empty; the model fields are omitted from JSON when empty). `Require` (~JAVMC41) is a `Check`
shape minus `waived_for` plus the source `message?` and `citation?`: a require is never
suppressed (obligations.md 4), so it carries no waiver state, and it reuses `Verdicts` so
per-alternate outcomes read the same as a check's. Its own digest covers path, expression,
message and citation (not verdicts) and enters the project digest like a check's. Objects are closed
(`additionalProperties: false`). A test regenerates the schema and
compares it to the committed file, and validates a fully populated
`Project` and each item entity against it, so a serializer change
without a schema change fails CI. `classify(old, new)` is the rule:
a new optional property or a new `$defs` entry is minor; a removed or
retyped property, a new required property, a changed enum or a removed
definition is breaking and bumps `cadetta.ir/N`. The committed file is
exactly `schema_json()`; the test does no file or environment access
(CAP001), so a stale file is replaced by that output.

## 6. Validation (`ir-validate`)

`cadetta.ir/1` documents are validated by the committed JSON Schema plus
the invariants below; the conformance entry `ir-validate FILE` loads a
document and exits 0 when it is valid, 2 when it is not (no diagnostic
code: an invalid document is not a design). Each invariant is stated
normatively in sections 1-4; this list is the checkable set, and
`tests/conformance/ir/` holds a must-fire (invalid) and must-not-fire
(valid) fixture for each:

- Exact: `r` is `num/den`, reduced, `den > 0`, integers as `n/1`; a
  magnitude is never a bare float; an `Inexact` names its `root`;
  `exact = false` carries a `reason`.
- Value: no `designed` field; `Dim` has every exponent as an integer and
  `currency` keys are uppercase ISO codes with integer exponents.
- Wire form: absent optionals are omitted (never `null`); enums are
  lowercase; wire names are `priv`, `type`, `for`; `sign` is the integer
  1 or -1; `imported` provenance is an object; digests are lowercase
  hex; `schema` is exactly `cadetta.ir/1`.
- Members: `provenance` is `derived` exactly when `deps` is non-empty
  (design flavour); `analysis` is true exactly for a `let`; a derived
  member has no `tol`; a parameterized component's members have no
  `value`; a `variants` value differs from the primary value; a `tol`
  term has `lo <= hi` and a canonical path as `var`.
- Bound: `<=` carries `hi` only, `>=` carries `lo` only, `in` carries
  both (decided here; section 3 listed the fields without saying which
  operator needs which).
- Paths: full form only; the item name starts uppercase; a component
  path has no member tail; indexes are integers and `[*]` never appears
  in the IR (an array placement lowers per instance); variant keys are full-form `SLOT=ALT`
  whose slot side carries a `/slot` step.
- Geometry: an `Op` is a one-key object; `flip` takes `x` or `y`;
  `rotate` takes `x|y|z` and an angle member path; a variant value of a
  datum or placement is the whole op chain, an array.
- Project: `modules` is sorted; `dependencies` is sorted by name.
- Affine: a `var` ending `.u` has a partner ending `.v` with the same
  stem in the same form, with equal `lo` and `hi`.
- Observed flavour: `frames[]` and `instances[]` may be non-empty (design
  IR has none); each transform is 3x4. One component `<package>::Observed`; every member is
  `imported`, has empty `deps`, a path-safe name (collisions `_2`,
  `_3`) and one binding with its locator; no citations, parts, fits,
  checks or solvers.
- Acyclic `deps` is D003 (a diagnostic, run through `lint`).
