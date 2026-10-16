# Adapters

An adapter connects the IR to one CAD tool or file format. Cadetta has no
favourite CAD tool: every target is an adapter, and the core never links
a vendor SDK.

## 1. Capabilities

Each adapter declares which of three capabilities it implements.

| Capability | Direction | What it does | Removes the manual step of |
|---|---|---|---|
| `emit` | IR -> file | Writes a parameter file the CAD tool links to or imports. | retyping values |
| `read` | file -> observed IR | Extracts parameters (global variables, equations, dimension values, custom properties) from native files. | transcribing existing designs |
| `drive` | IR <-> live tool | Opens documents, pushes values, triggers regeneration, reads results back and saves. | open, import, rebuild, check |

`drive` implies the adapter can report regeneration failures (rebuild
errors, failed features) as diagnostics (R004).

## 2. The loop

```
cadetta sync = compile -> lint -> read -> classify -> push -> regen
               -> read back -> record
```

`sync` is a three-way compare per locator:

- **base**: the lock's `synced` record for the locator, the value
  cadetta last pushed (or adopted) and read back. The records live in
  the committed `cadetta.lock` (obligations.md 3), not in a gitignored
  file: the base describes committed CAD files, and two branches that
  sync one locator differently are a real conflict.
- **ours**: the value freshly computed for the locator (the emit point,
  quantities.md 5.6, in the locator's native unit at the declared
  precision).
- **theirs**: the value `read` finds in the native document before any
  push.

| ours vs base | theirs vs base | Class | Action |
|---|---|---|---|
| same | same | no-op | nothing |
| differs | same | push | push `ours` |
| same | differs | cad-moved | R001 "cad moved"; no push; exit 1 |
| differs | differs, theirs = ours | converged | record only |
| differs | differs, theirs != ours | both-moved | R001 "both moved"; no push |
| no base | theirs = ours | adopt | record only |
| no base | theirs != ours | first push | push `ours` (R006 if it replaces an expression) |

R001 states which side moved and shows base, ours and theirs.
`sync --force-design` pushes `ours` over any R001 class.

`sync` is idempotent: when every locator classifies as no-op or
converged it writes nothing, triggers no regen and does not touch any
document. The no-op comparison is over emitted lines or values; the
header line of an emitted file (which carries the IR digest and changes
on any unrelated edit) is excluded.

Without `drive`, `sync` degrades to `emit` plus instructions for the
user's one manual step (relinking or reloading). An external adapter that
declares `emit` but not `drive` is sent the `emit` call (design IR, its
bindings, the emit policy) and the files it reports are listed as emitted.
There is no `theirs`, so no classification happens. `read` on a later run detects drift only
if the adapter can read the live model.

**CAD file reflects design.** Only a `drive` adapter (or a `read` of the
native document itself) discharges this obligation (obligations.md 2).
The `sw-equations`, `json` and `csv` adapters never do (none has `drive`):
the file they read is the file cadetta wrote, so the obligation stays open with the reason
"unverifiable without drive", and `sync` reports it as O004 (info by
default). The user may attest it with a role (`ack`).

### 2.1 Cadetta owns the dependency structure

`sync` replaces native expressions on bound locators with values or with
links to cadetta-emitted globals. R006 (warning) fires whenever a push
replaces a native expression or driving equation, listing the old
expression. A push to an equation-driven dimension removes the equation
and then sets the value; the removal is reported, and `--dry-run` shows
it first. `import` keeps such expressions as `.cdt` references (7), so
after `--accept` the structure lives in the source.

### 2.2 Documents, sessions and saving

- `drive` attaches to a running instance of the tool if one exists,
  otherwise starts one hidden. It closes only what it opened.
- A document that is dirty, locked by another process or read-only is
  R009; it is refused before any push for that document.
- After a successful regen and a matching read-back, the adapter saves
  the documents it modified (`[[adapter]] save = "after-sync"`, the
  default; `"never"` leaves them modified and the sync obligation open
  with the reason "unsaved").
- The lock records the digest of the saved file. `read` of a file whose
  on-disk digest equals the lock's skips opening it in the tool and
  reuses the recorded values.

### 2.3 Failure handling and atomicity

The host side of R008-R010 lives in `cadetta_adapter::availability`: an
unreachable, version-mismatched or wrong-platform adapter (its `initialize`
`platforms` list, when non-empty, must name the host's `linux`, `macos` or `windows`)
is R008 (error with exit 2
under `sync`; info under `check`, which reports the R lints of the adapter's
`read` and, when the adapter cannot start, only the info R008);
each dirty or read-only document is an R009 of its own and is never
touched (when every document is refused the host still sends `close` and
`shutdown`); rejected locators are grouped into one R010 per file while the
other locators proceed.

| Failure | Code | Behaviour |
|---|---|---|
| tool not installed, wrong platform, adapter version mismatch | R008 | `sync` fails, exit 2; `check` reports info and uses the lock base as observed |
| document dirty, locked, read-only | R009 | refused before any push |
| driven or suppressed dimension, equation or unit conflict | R010 | that locator is rejected, others proceed |
| rebuild error after push | R004 | the document is restored (below) |
| value not representable at declared precision | Q004 | value withheld, path not written, exit 1 |
| two emitted paths create one native name | R013 | hard failure before any write |
| a bound `Text` parameter must be emitted to a format that cannot express `Text` | R014 | that adapter's file withheld, exit 1 |
| a dimension, property or frame locator, or a `ref` binding, must be emitted to a `json` or `csv` file; a property, frame or instance locator must be emitted to `sw-equations` | R015 | that adapter's file withheld, exit 1 |
| a `csv` file has a row whose name matches no binding | R016 | no observed IR for that file, exit 1 |
| bound path missing natively | R003 | locator skipped |

Pushes are per document. Before pushing, an adapter MUST snapshot the
current values of every locator it will change in that document and
restore them on R004 (SOLIDWORKS: rebuild fails, set the old values,
rebuild again). If the restore itself fails the adapter says so in the
diagnostic. Lock `synced` records are updated only for locators whose
read-back matched, so a partial sync reconverges on the next run.

The host implements this in `cadetta_adapter::atomic`: `push_atomic`
snapshots, writes, rebuilds and restores on R004 over a `Document`, and
`matched_locators` selects the locators eligible for a `synced` record.
`sync` pushes every file through `push_atomic`, since every backend exposes
the file as a `Document` (`Backend::document`), and records a `synced` entry
only for the locators `matched_locators` returns.

### 2.4 Resolving drift

| Direction | Command |
|---|---|
| design to CAD | `cadetta sync --force-design` |
| CAD to design | `cadetta import ADAPTER FILE... --update` (7.4) |

After either, the next `sync` classifies the locator as no-op.

## 3. Bindings

A binding maps a canonical path to an adapter's native locator, e.g.
`acme.rover::Rover/axle[*].end_diameter` to the SOLIDWORKS global
variable `"Axle_end_diameter"` or to the dimension
`"D1@Boss-Extrude1@Axle.SLDPRT"`.

- Bindings live in `bindings/<id>.toml`, committed, one table per
  canonical path; `<id>` is the adapter's `id` (3.2). `[[adapter]]
  bindings = "..."` points an adapter at another file, so two adapters
  for one tool (sw-equations and solidworks) share one. A path may bind
  several native locators (a legacy file often drives one value through
  a global and a dimension, or two globals); each locator has a `kind`
  (`global`, `dimension`, `property`, `frame`, `instance`).
  `#[native(adapter, "locator")]` in source pins one inline. The file
  holds only what the native documents cannot say themselves: native
  links (3.5) bind without an entry. The file spelling:

  ```toml
  ["acme.gearbox::CoverPlate.thickness"]
  locators = [
    { locator = "Plate_Thk", kind = "global", mode = "value", native_unit = "mm" },
    { locator = "plate thickness", kind = "global", mode = "value", native_unit = "mm" },
    { locator = "D1@Boss-Extrude1@CoverPlate.SLDPRT", kind = "dimension", ref_to = "Plate_Thk" },
  ]
  ```

- Locator fields:

  | Field | Meaning |
  |---|---|
  | `locator` | the native name or dimension spelling |
  | `kind` | `global`, `dimension`, `property`, `frame` (a SOLIDWORKS coordinate system or plane feature), `instance` (a component instance in an assembly, for a placement) |
  | `mode` | `ref` or `value` (below) |
  | `native_unit` | unit of the native number |
  | `ref_to` | for a `ref` dimension, the global it links to |
  | `for` | an alternate of an interface-typed slot (3.3) |
  | `config` | the configuration the locator belongs to; import records the active one (7.3) |
  | `sign` | `-1` negates a coordinate on write and read (geometry.md 6) |
  | `component` | in-assembly instance (`"wheel<2>"`, emitters/sw-equations.md 4) |
  | `tolerance` | `drive` or `ignore`: whether a `dimension` locator's tolerance is written (3.6); default `[[adapter]] tolerances` |

- Locator validation (`cadetta-adapter` `locator.rs`): `locator` is non-empty;
  `ref_to` needs a `dimension` locator in `ref` mode; `sign` does not apply to
  a `property`; `for` and `config` are non-empty when present; `component` has
  the form `name<N>`; `tolerance` applies to a `dimension` only.
  Native names are namespace-qualified by kind
  (`global:`, `dimension:`, `property:`, `frame:`, `instance:`), so one name in two
  namespaces is not R013; the same qualified name from two paths in one
  configuration and component is.

  A bindings file or locator that fails these rules is a configuration
  error, not a lint: the run stops with exit 2 naming the file path and
  the offending key, and carries no diagnostic code.

- Parameterized components exist only as instances. A binding may name
  one instance (`Rover/axle[1].end_diameter`) or all of them
  (`Rover/axle[*].end_diameter`). A `[*]` binding is emitted once, at
  instance 0. Every instance of a part array has the same arguments
  (language.md 3.5) and tolerance variables share the nominal, so
  today all instances emit the same value and `[*]` cannot diverge.
  R012 (error) is reserved for a future language feature that gives
  instances differing arguments or index-dependent members: if any
  instance's emitted value then differed from instance 0, the user
  would bind the instances separately. No source reaches R012 now.
- Only bound paths are emitted, explicitly or through a native link or
  `bind = "scope"` (3.5); an unbound parameter is not written. An
  adapter that cannot read discovers no links, so it emits explicit
  bindings and `bind = "scope"` globals only.
  `let` values are never emitted (quantities.md 5.3).
- Emitted values are values (the emit point, quantities.md 5.6), never
  expressions: the dependency structure lives in cadetta, and CAD tools
  receive numbers or links to cadetta-emitted globals (2.1).
- Unbound native parameters are R002; bound paths missing natively are
  R003 (including a native link that has a lock base and is no longer
  found, so deleting a global in CAD is never silent); bindings matching
  no path are R005. `was` paths carry bindings forward across renames
  and moves.
- A locator's `mode` is `ref` (default where the target supports
  references: the native side keeps a live link to a cadetta-emitted
  global) or `value` (a flat number). A `ref` dimension points at its
  `ref_to` global if given, else the first global locator bound to the
  same path (bindings-file order); if the path has none, at the
  global of its referent when the path is a pure alias; otherwise the
  emitter creates the path's own mangled global (3.1). That created
  global is a `value` global whose unit is the dimension locator's
  `native_unit`, or the adapter's declared native unit when the locator
  has none; it is written before the path's dimension lines.
- Sync of a `ref` locator: its value is carried by the target global, which
  syncs as a value, so the dimension itself is never pushed or compared (a
  push would replace the link, R006). `sync` instead checks the link: a `ref`
  dimension absent from the CAD files, or whose native expression no longer
  names its target global, is R003 (a named target must appear in the
  expression; an alias or created global only needs an expression present).
- File defaults (`cadetta-adapter` `bindings.rs`): `mode` defaults to
  `ref` for a `dimension` locator and `value` otherwise; an omitted
  `native_unit` means the adapter's declared native unit. A
  `#[native(adapter, "locator")]` pin replaces the file entry for its
  path with one `global`, `value` locator (several pins for a path
  accumulate). A `was` rename moves the binding unless the new path is
  already bound. `[*]` matches any index when testing R005.
- Units at read: a native value without a unit takes `--assume-unit` when
  the run gives one (it wins over the locator's `native_unit`), else the
  locator's `native_unit`, else the adapter's declared native unit
  (`initialize`), recorded in observed IR as
  `unit_assumed`. Formats with no unit information at all need
  `--assume-unit` (on `import` and `ir --observed`); without it the
  value is read with `exact: false` and a reason. Read never raises
  Q001; Q001 is a source error.

### 3.1 Naming and mangling

A name the adapter creates natively (a global with no bound locator, or
a dimension's own global) is derived from the canonical path:

1. drop the package and module path;
2. replace `.` and `/` with `__` (`__` is reserved, N005);
3. replace `[i]` with `_i`; a `[*]` segment is dropped.

`Rover/axle[1].end_diameter` becomes `Rover__axle_1__end_diameter`;
`Rover/axle[*].end_diameter` becomes `Rover__axle__end_diameter`; the
datum `Rail.site` becomes `Rail__site` and its offset
`Rail__site__offset__x`; the placement op `Mount/bracket.place.offset[1].x`
becomes `Mount__bracket__place__offset_1__x`. The rule is the same for
every adapter and every target, whether or not the target allows dots,
so switching adapters never renames a global. `[[adapter]] prefix`
optionally prepends a string to every created name.

The name is a pure function of the path and the prefix. Adding,
removing or renaming any other path never changes it, so a link made in
the CAD tool today survives every later edit except a rename of the path
itself, and that is covered by `was` (3.5). `cadetta names` prints the
name of every in-scope path (config-cli.md 3); tools never reimplement
the rule.

The rule is not injective, because it drops the package and module. Two
in-scope paths of one adapter with the same created name are R013, a
hard failure before any write; the user resolves it by binding one of
them explicitly (`#[native(adapter, "Name")]` or the bindings file),
which takes it out of the created names. Nothing is qualified
automatically, since that would rename an existing global when an
unrelated path is added. Read maps native names to paths through
bindings and the forward table of 3.5, never by demangling.

### 3.2 Adapter identity and scope

`[[adapter]] id` names an adapter instance; it is required when `name`
repeats and defaults to `name`. The id keys the bindings file
(`bindings/<id>.toml`) and the lock records. `modules = [...]` (module
path prefixes) or
`paths = [...]` (canonical-path globs) restrict what the adapter emits,
reads and syncs, so two products can each link their own equations
file. `sync`, `emit` and `check` accept `--module PATH` and path globs
with the same effect for one run.

A `modules` entry uses the same prefix form as `--module`
(config-cli.md 3, "Scoping"): a module path relative to the package
(`gear`, `gear::sub`) or fully qualified (`acme.demo::gear`), compared
segment-wise, so `gear` covers `acme.demo::gear::X.y` and
`acme.demo::gear::sub::X.y`, not `acme.demo::gearbox::X.y`. A bare
package name is a package prefix and matches on dot boundaries
(`acme.gear` covers packages `acme.gear` and `acme.gear.sub`, not
`acme.gearbox`). Module paths follow the file tree (packages.md 2), so
`gear::sub` is `src/gear/sub.cdt`; module files stay `snake_case`
(E015) and a dotted file name is never a module path.
In
`paths` globs, `*` matches any run of characters without a `:` (it stays
inside one module or item segment) and `**` matches any run. A path
reaches the adapter only if it passes both filters (an empty list means
all); per-run flags narrow the adapter's scope, never widen it.

### 3.3 Interface-typed slots

A binding for a path under an interface-typed slot carries
`for = "<alternate>"` (a qualified component name, e.g.
`for = "acme.shop::Colson2Series"`). Only locators whose `for`
matches the slot's primary (or locators with no `for`) are active; a
swap changes which locator set is active. `read` of an assembly records
the file each component instance uses; R011 (warning) fires when the
file instantiated is not the file bound to the slot's primary.
`impact --swap` lists the replace-component action (obligations.md 5.1).

### 3.4 Geometry bindings

Geometry is bound through members and frames, never through derived
coordinates, and positions are verified by transform (geometry.md 6):

| Source form | Native counterpart | Binding | Verified by |
|---|---|---|---|
| datum | a frame: coordinate system (preferred) or plane feature | `frame` locator, explicit or by name (3.5) | the frame's transform against the composed datum |
| datum offset or rotation Member | a dimension of the feature that defines the frame | `dimension` locator | the dimension (R001) |
| hand-listed point coordinate | a sketch dimension measured from the datum | `dimension` locator, `sign` when the native value is a magnitude | the dimension; R007 |
| pattern generator member (pitch, radius, start, rotation, count) | the native pattern's dimension or instance count | `dimension` locator (a count is a Count) | the dimension |
| placement | the component instance | `instance` locator: the instance name, or by reference property (3.5) | the instance's transform against the composed placement |
| placement offset or non-90deg rotation Member | a mate distance or angle dimension | `dimension` locator | the dimension |
| `flip`, exact 90deg `rotate` | none | none | the transform comparison only; never pushed |
| array placement over a pattern | a pattern-driven component pattern | none (it follows the part's pattern) | each instance's transform |

- Transform comparison. `read` reports frames and instances with their
  transforms (ir.md 1). A datum's frame transform is compared with its
  composed transform at the emit point. A placement is compared as the
  placed datum's frame in the assembly (the instance transform times
  the part's datum frame, read when the part's datum is bound, else
  composed) against the anchor's composed frame. An entry matches when
  `|design - read| <= 0.5 * 10^-p`, `p` being the adapter's precision: translations in the native unit,
  rotation entries as plain numbers; an inexact design entry uses its
  nominal. A mismatch is R001 on the datum or placement path, showing
  both transforms.
- Composition. A datum's frame is its parent's frame followed by its
  ops left to right, each op applied in the running local frame:
  `offset` translates, `rotate` uses the angle Member, `flip` is a half
  turn. `<Type>.origin` is the identity. Translations are divided by the
  adapter's native unit. A placement's anchor frame is the frame of its
  `at` datum followed by the placement ops. When `at` is a datum of
  another part, that part is the unique placement in the same component
  whose placed datum has that type (matching the array index for an
  indexed placement); its node frame is its anchor frame times the
  inverse of its placed datum's frame. An ambiguous or missing node is
  skipped with a warning, never guessed. Variants compose with the base
  ops.
- A placement is verify-only: it is never pushed, so any mismatch is
  R001 whatever the base (move the part in CAD, or change the design).
- A datum is verify-only under `[[adapter]] datums = "verify"` (the
  default). Under `"create"` the adapter creates a missing coordinate
  system named by the datum's mangled path in the datum's component
  document, defined by numeric values relative to the document origin,
  and `sync` pushes the composed transform into it like a value, with
  a lock `synced` record. Then the datum's offsets need no dimension
  bindings at all.
- `cadetta sync` runs this pass after every `read`, dry runs included:
  the observed `frames` and `instances` are compared with the composed
  design and each mismatch is an R001 in the sync report (a blocking
  diagnostic). Under `datums = "create"`, and only when the adapter
  announced the `create_frames` feature, the datum entries are added to
  the sync entries and travel the same three-way loop as values. A
  datum's document is the file of an observed member of its component,
  else the document of its bound frame, else the adapter's only
  configured file.
- Coordinates derived from a generator are analysis-only unless the user
  lists them in the bindings by hand. R007 covers a dimension not
  measured from the sketch origin.
- The table above is executable: `cadetta_adapter::geometry_plan` maps a
  source form to its native binding, whether `sync` may write it, and
  what `read` reports on drift (R001 for a placement, a drift note for
  `flip` and exact 90deg `rotate`). `sign = -1` negates the written
  value and restores it on read.

### 3.5 Native links

A bindings-file entry that restates what the native document already
says is duplication. Four native facts are therefore bindings by
themselves. The host records them with `implicit` on the locator (ir.md
3) and never writes them to the bindings file:

| Native fact | Binds | Locator |
|---|---|---|
| a global whose name is the created name (3.1) of an in-scope path | that path | `global`, `value`, implicit `global` |
| a dimension whose native equation is exactly a reference to such a global (`"D1@Boss-Extrude1" = "Bracket__thickness"`) | the same path | `dimension`, `ref` with `ref_to` that global, implicit `link` |
| a frame whose name is the created name of an in-scope datum (`Rail__site`) | that datum | `frame`, implicit `frame` |
| a component instance whose reference property is a part's instance path relative to its assembly (`bracket`, `feet[2]`, `sub/bracket`) | that part's placement node (`Mount/bracket.place`) | `instance`, implicit `reference` |

- Discovery runs during `read` over the adapter's `files`. The host
  builds the forward table of created names for the in-scope paths and
  their `was` paths and looks native names up in it; it never parses a
  native name back into a path. An adapter that cannot read
  (`sw-equations`, `json`, `csv`) discovers nothing.
- A native name matching a `was` path binds the current path. Where the
  adapter can rename a global and its references (`initialize` feature
  `rename_globals`), `sync` renames it to the current created name and
  reports R006; otherwise it keeps writing the old name.
- An explicit locator of the same kind and native name replaces the
  implicit one; an explicit locator naming a different native item adds
  to it (multi-locator, 3). Several implicit locators for one path are
  normal (a global in each document, many dimensions linked to one
  global) and all are active. Two instances whose reference property
  names the same part are R013.
- When native names match both a path's created name and a `was` name
  of it in one document, the current name binds and the old one is R002.
- The created names, and so R013 for them, depend only on the source;
  discovery adds locators but never creates a name.
- A dimension whose equation is anything other than a bare reference
  (`= 2 * "X"`, `= "X" + 1mm`) is not a link; it is R002 like any
  unbound native item, with the expression in the message.
- Implicit locators take part in the three-way sync and in lock records
  exactly like explicit ones. They are never R005; one that disappears
  after a sync recorded it is R003 (3).
- `[[adapter]] bind = "scope"` also binds every in-scope path whose
  kind the adapter can express to its created global (implicit
  `global`), whether or not the global exists yet, so `emit` and `sync`
  create it. That is how an equations-file user gets globals to link
  dimensions to inside the CAD tool. The default, `"explicit"`, binds
  only bindings-file entries, pins and discovered links. Scope binding
  covers parameters, datum and placement op Members, hand-listed
  coordinates and generator members of the adapter's own scope
  (`modules`, `paths`); never a `let`, never another package's member,
  and never a `Text` path for an adapter that cannot express `Text`
  (R014 stays a rule about explicit bindings).

### 3.6 Dimension tolerances

The tolerance on a drawing belongs to the number cadetta owns, so it is
written from the design instead of retyped. A `dimension` locator's
`tolerance` is `drive` or `ignore`, defaulting to `[[adapter]]
tolerances` (`"drive"`). It has effect only in adapters whose
`initialize` reports the feature `tolerances`.

- With `drive`, `push` writes the path's design tolerance together with
  the value: `[+-t]` as a symmetric tolerance, an asymmetric bracket as
  bilateral deviations, an interval value as limits, and an ISO 286
  class (`12mm H7`, a fit side) as the tool's fit tolerance naming the
  class, or as bilateral deviations where the tool has none. An
  untoleranced path writes no tolerance.
- When the emit point is not `nominal` (quantities.md 5.6), the value
  written is not the class's basic size, so the tolerance is written as
  limits (`.min`, `.max`) instead of a class or deviations.
- A dimension in `ref` mode takes its tolerance on the dimension, never
  on the global (globals carry none).
- `read` reports a dimension's tolerance, and `sync` compares it in the
  same three-way record as the value: a tolerance edited in CAD is R001
  like an edited value.
- A position zone (geometry.md 3.1) is not a dimension tolerance and is
  never written.
- `cadetta sync` drives this: for a `value`-mode quantity `dimension`
  locator under `drive`, the design tolerance is pushed with the value
  and the tolerance `read` reports (in `tolerances`) is the CAD side of
  the three-way compare. The design IR keeps no fit class or interval
  spelling, so `sync` writes `[+-t]` as symmetric and any other spread
  as bilateral deviations (never limits or a class). A tolerance read
  back that differs from the one pushed fails the read-back compare.
- Before comparing, `sync` converts an observed value or tolerance in
  another unit of the same dimension to the locator's native unit when
  the conversion is exact (`0.5in` equals `12.7mm`); anything else
  (other dimension, affine unit, pi, non-terminating) is compared as
  text.
- `sync` does not yet drive `ref`-mode locators; they are skipped.

## 3a. What every emitter spec states

Each built-in emitter has a format document in `docs/spec/emitters/`
([sw-equations.md](emitters/sw-equations.md), [json.md](emitters/json.md),
[csv.md](emitters/csv.md)) fixing: a header carrying the IR digest and
adapter version; line order (the topological order of
emitters/sw-equations.md 2);
globals vs dimension lines and refs vs values per binding mode; the
emitted point and unit; the default output path; and tolerance and
citation as a trailing native comment where the format has comments
(`"Rover__axle__end_diameter" = 12.029mm  ' 12mm p6`). Every emitter
writes the same content in the same order, separated by single spaces:
authored literal with tolerance or class, `emit=POINT` when the emit
point is not `nominal`, then the citation; `json` and `csv` put it in
their `note` field. Drift
(R001) compares the read-back against the emitted point, never the
nominal.

## 4. Contract every adapter MUST meet

1. **Emitters decide nothing.** Output is a pure function of (IR digest,
   bindings, adapter version, adapter config). Any choice an adapter
   seems to need (which tolerance point, which unit) is decided upstream
   by policy and arrives in the request.
2. **Deterministic bytes.** Same inputs, same output bytes. Golden-file
   fixtures for write-only formats; round-trip fixtures (emit, read,
   compare IR values) for formats the adapter can also read.
3. **Exactness is reported, not hidden.** Converting a rational to the
   target's native number format and unit happens in the adapter at a
   declared precision; if the value is not representable within that
   precision, the adapter returns Q004 with the error, never rounds
   silently.
4. **Read is lossless or says so.** A value the adapter cannot extract
   faithfully (driven dimension, expression it cannot parse) appears in
   observed IR with `exact: false` and a reason, not as a guess.
5. **No source of truth outside the IR.** An adapter never writes back
   into `.cdt` source; `import` produces a proposal the user accepts.
6. **Failures are structured.** Every refusal or error is a
   `cadetta.diag/1` record (lints.md 1.1) with the codes of 2.3; a
   `push` returns a status per locator, never only a summary.

## 5. Protocol

Built-in adapters run in process (Rust). External adapters are separate
executables, written in whatever language reaches the CAD API (C# for
SOLIDWORKS COM, Python for FreeCAD and Fusion), speaking JSON-RPC 2.0
over stdio:

| Method | Params | Result |
|---|---|---|
| `initialize` | protocol version, client info | name, version, capabilities, features, native units, precision (the emitter spec's default, overridable by `[[adapter]] precision`), platforms |
| `emit` | IR (design), bindings, policy | files written (paths, digests), diagnostics |
| `open` | files | `session` (handle, per-file dirty/readonly state), diagnostics |
| `read` | file paths, or a `session` | observed IR, diagnostics |
| `push` | `session`, `file`, changed params (path, value, locator, `mode`) | per-locator status (`pushed`, `skipped`, `rejected`, `failed`; old and new value), diagnostics |
| `regen` | `session`, targets | per-target status, rebuild diagnostics |
| `save` | `session` | saved files (paths, digests), diagnostics |
| `close` | `session` | - |
| `shutdown` | - | - |

`open`, `save` and `close` are required of `drive` adapters only;
`read` without a session opens and closes its own. `mode` tells the
adapter how to write a dimension (`ref`: create or keep the link to
`ref_to`; `value`: set the number; `frame`: set a coordinate system's
3x4 transform, `value` being three rows of four numbers, creating the
system named by the locator when missing, to `create_frames` adapters
only). `initialize` reports the adapter
version; if it does not match `[[adapter]] version`, the adapter is
unavailable (R008, exit 2); so is one whose `platforms` exclude the host's, and one whose
`push` answers with a summary instead of a status per locator. The precision in force is
`[[adapter]] precision` when set, else the one `initialize` reported (`cadetta sync` applies
this through `Client::effective_precision`; views that never spawn an adapter, such as
`names` and the no-spawn emitted-values view, use the emitter default). Built-in adapters
(`csv`, `json`, `sw-equations`) declare `emit` and `read`; a `drive` request to one is a
configuration error (exit 2). Adapters are located by `command` in
`cadetta.toml`; there is no PATH search (a bare name such as `sh` names a file in the
package directory, never a system program). `check`, `sync` and `import` kill an adapter that leaves a call
unanswered for `[[adapter]] timeout` seconds (default 60); `check` reports R008 and
`sync`/`import` fail with R008 (exit 2). Diagnostics in every result use
`cadetta.diag/1`.

Adapters are declared in `cadetta.toml` (`[[adapter]]`, config-cli.md
2); the protocol schema lives in `docs/schemas/` beside the IR schema.

Wire details fixed by the host (`cadetta-adapter`, `rpc`):

- Framing is newline-delimited: one JSON-RPC message per line on the
  adapter's stdin and stdout. Stdout carries protocol only; an adapter
  logs to stderr, which the host passes through. A message without an
  `id` from the adapter is a notification and is ignored.
- `initialize` params are `{protocol: "cadetta.adapter/1", client:
  {name, version}}`; `capabilities` is a list of `emit`, `read`,
  `drive`; `features` is a list (possibly empty) of `frames` (reads
  frame transforms), `instances` (reads component transforms and
  reference properties), `create_frames` (creates and drives coordinate
  systems, 3.4), `tolerances` (reads and writes dimension tolerances,
  3.6) and `rename_globals` (3.5); `precision` is the number of
  fractional decimal digits of the native number format.
- `read` takes `{source: {files: [...]}}` or `{source: {session:
  handle}}`. `regen` per-target status is `ok` or `failed`.
- The host refuses `open`, `save`, `close`, `push` and `regen` unless
  `drive` was declared, and `emit` unless `emit` was.
- A drive sync runs `open`, `push` per file, `save` (per `[[adapter]]
  save`; skipped when `never` or when any locator `failed`), `close`,
  `shutdown`, in that order. `open` reporting a dirty or read-only file
  is R009: the session is closed and nothing is pushed. Push status is
  per locator.
- Every result is parsed against the types in `docs/schemas/adapter-protocol.json`
  (unknown fields are refused); each diagnostic must be a `cadetta.diag/1`
  record.

## 6. Target matrix

Planned support, not a 0.1.0 promise beyond the rows marked 0.1.0.
`drive` includes pushing into live documents, so there is no separate
SOLIDWORKS `emit`: writing a text file for a part to link is the
built-in `sw-equations` row.

| Target | Kind | emit | read | drive | Mechanism |
|---|---|---|---|---|---|
| JSON | built-in | 0.1.0 | 0.1.0 | - | parameter list (emitters/json.md) |
| CSV | built-in | 0.1.0 | 0.1.0 | - | name, value, unit rows (emitters/csv.md) |
| SOLIDWORKS equations file | built-in | 0.1.0 | 0.1.0 | - | text file the part links via "Link to external file" |
| FreeCAD | external, Python | 0.1.0 | 0.1.0 | 0.1.0 | Spreadsheet aliases; headless `freecadcmd` for CI |
| SOLIDWORKS | external, C# | - | 0.1.0 | 0.1.0 | COM API from .NET: global variables, dimensions and their tolerances, custom properties, coordinate systems, component transforms, rebuild (push covers emit); plus an add-in for linking by selection (solidworks.md); Windows only |
| Onshape | external | - | later | later | REST: variable studios, part studio variables |
| Fusion | external, Python | - | later | later | user parameters API |
| Inventor | external | - | later | later | parameters API |
| OpenSCAD / CadQuery | built-in | later | - | - | `-D` defines / Python constants |
| KiCad | - | out | out | out | electrical CAD is out of 0.1.0 |

SOLIDWORKS is the primary target: it is what the first users design
in. Its adapter is one C# solution, a protocol executable and a
SOLIDWORKS add-in sharing one library, because an in-process add-in
must be .NET and two code bases against one COM API would duplicate
each other (solidworks.md). The Python `cadetta.adapter` base
(bindings.md 3.4) serves FreeCAD and other Python-reachable tools. CI
cannot run SOLIDWORKS, so its tests replay recorded COM transcripts
against a fake, and a release requires a manual smoke run on a licensed
machine (the `sync` loop on the pilot r3-05 part). The built-in
equations-file emitter remains the zero-install path.

FreeCAD is the reference external adapter for CI: it is the only target
that exercises `read` and `drive` without a license or Windows, so the
protocol's conformance tests run against it.

### 6.1 FreeCAD emit

`emit` writes one Spreadsheet object named `cadetta` into the FCStd
document at `policy.out` (opened, or created when missing; the sheet is
replaced whole, nothing else in the document is touched). The adapter
reaches FreeCAD only through a backend seam, so the cell plan is a pure
function of the request.

- **Request.** `bindings` is the list of `ir.Binding` records this run
  emits (the host has already restricted it). `policy` is an object with
  unknown keys rejected: `out` (required FCStd path), `precision`
  (default 6), `point` (default `nominal`; a member's `emit` attribute
  wins), `paths` (optional canonical-path filter), `names` (path to
  created mangled name, supplied by the host so tools never reimplement
  mangling, 3.1), `alias_of` (path to the referent path whose global a
  `ref` locator follows) and `native_unit` (default `mm`). A malformed
  policy, or a design IR without a digest, is a JSON-RPC error -32602.
- **Layout.** Row 1 is the header: `A1` `cadetta emit freecad VERSION`,
  `B1` `ir=blake3:HEX`, `C1` `bindings=blake3:HEX`. Each later row has
  the canonical path in `A`, the value or formula in `B` carrying the
  alias, and the note in `C` (authored literal when it differs from the
  written value, `emit=POINT` when not nominal, then the citation).
- **Values.** A dimensioned value is the formula `=DEC UNIT`
  (`=12.029 mm`), a Count or Ratio the bare number. `DEC` is exact at
  `precision`, trailing zeros trimmed; a value not representable there,
  or an inexact one (Q006), is Q004 and is never rounded. `sign = -1`
  negates the written value (zero stays unsigned). `mmc` or `lmc` on a
  member without a hole or shaft role is E010.
- **Locators.** `global` locators and `dimension` locators are written
  as alias cells, globals first. In emit a `dimension` locator is a
  Spreadsheet alias; driving a sketch constraint is `drive`'s job.
  A `ref`-mode dimension is the formula `=GLOBAL`, where GLOBAL is
  `ref_to`, else the path's first global locator, else the first global
  locator of the `alias_of` referent, else the path's own created global
  (`policy.names`), written once as a value cell before its dimension
  rows. `property`, `frame` and `instance` locators and locators with
  `for` set are not written by emit. A `[*]` binding emits instance 0.
- **Order.** Paths are written in dependency order, the smallest
  canonical path first among the ready ones, following dependencies
  through unbound members (emitters/sw-equations.md 2); a cycle is D003.
- **Failure.** A diagnostic of error severity (Q004, E010, D003, R013
  for a repeated alias, R005 for a bound path missing from the design,
  R015 for an alias that is not a valid Spreadsheet alias, collides with
  a cell address or has no resolvable `ref` target) withholds the whole
  document: nothing is written and `files` is empty.
- **Determinism.** The reported digest is the blake3 of the canonical
  cell plan (cell, alias, content per line), not of the FCStd bytes: a
  saved FCStd is a zip that FreeCAD stamps with the save time, so it is
  the one place 4's byte-determinism rule applies to the plan instead of
  the file.

### 6.3 FreeCAD drive

`drive.py` runs sessions over the backend seam. `open` attaches to a
document already open in the running instance, else opens it hidden, and
reports `dirty` (unsaved changes) and `readonly`; either is R009 and the
file is refused: `push` to it rejects every locator without writing. A
locator is a Spreadsheet alias or a `Sketch.Name` dimension. `push`
snapshots each locator it will change, skips an unchanged value (no
write) and a missing locator (R003), rejects a driven dimension or a
non-numeric value (R010), and warns R006 when it replaces a native
expression. `regen` recomputes the targets (every session file when
empty); a failed object is R004: the snapshot is written back and the
document recomputed, and the diagnostic says whether the restore held.
`save` writes only modified files whose rebuild did not fail and returns
`blake3` digests of the saved bytes. `close` closes only documents the
adapter opened. `read` accepts a `session`, reading the open documents.
The live FreeCAD calls are not exercised in CI without `freecadcmd`.

## 7. Import

`import` is the adoption path for existing CAD files. It never writes
`.cdt` or bindings without an explicit flag, and it never overwrites.

### 7.1 Propose and accept

```
cadetta import ADAPTER FILE... [--assume-unit UNIT]    # propose
cadetta import --accept                                # apply
```

The first form reads the files (`read`) and writes a proposal:
`build/import/proposal.toml` and `build/import/proposal.cdt` (the
scaffold). The proposal is one `[[item]]` table per native item, sorted
by `file` then `locator`, with keys `file`, `locator`, `kind`
(`global | dimension | property | frame`), `native_name`, `expression`
(the native expression, when there is one), `value` (a source literal),
`path` (the proposed canonical path), `type` (the proposed component),
`status` (`accept | rename | skip`) and `reason` (for `skip`). The user edits
`proposal.toml`. `--accept` writes `bindings/<id>.toml` and
`cad/imported/<File>.cdt` from it. A re-run adds only locators not yet
bound and reports the others as unchanged; existing files are never
overwritten.

### 7.2 Naming

- Component: the file stem, UpperCamel (`cover_plate.SLDPRT` ->
  `CoverPlate`); one component per native file.
- Member: snake_case of the native name with a trailing unit suffix
  stripped (`Plate_Thk` -> `plate_thk`; `Bore_mm` -> `bore`;
  `plate thickness` -> `plate_thickness`), case per N001-N004.
- A collision is suffixed `_2`, `_3` and flagged in the proposal.
- A native expression `"A" = "B" * 2` becomes the `.cdt` expression
  `a = b * 2`; the references are to the proposed members.
- Assemblies: a `.SLDASM` becomes an `assembly` with one `part`
  declaration per distinct component file; instance bindings carry
  `component` (3).
- Values are imported as nominal scalars. Native tolerances are not
  recovered in 0.1.0. Driven (reference) dimensions are skipped with a
  reason.
- Imported positions and distances are scalars. Converting a set of them
  to a `Point2` array is a manual refactor that keeps the existing
  locators by re-keying the bindings (geometry.md 6).
- Frames and instances are derived, not retyped. An adapter with
  `frames` proposes each coordinate system as a `datum` built from
  `origin` by one `offset` and the fewest `rotate`s (whole multiples of
  90deg are recognised and written exactly; any other rotation is
  written as read and flagged). An adapter with `instances` proposes a
  `place` per component instance, anchored at the parent's `origin` with
  the read transform as its ops, and flags it for the user to re-anchor
  on a real datum. `--accept` binds them with explicit `frame` and
  `instance` locators naming the native items as they are, so no CAD
  file is touched (contract 4.5).

### 7.3 Configurations and properties

- Configurations: only the active configuration is imported and
  recorded as `config` on each locator; an info diagnostic lists other
  configurations when they exist.
- Custom properties bind as `Text`, `Count` or `Quantity`; a quantity
  needs an explicit `native_unit` in the proposal. Properties that are
  `$PRP`-linked or equation-linked are skipped with a reason.

### 7.4 Adoption marker and updating from CAD

`--accept` writes a lock record `adopted <adapter-id> <file>`; the first
`sync` that touches the file clears it. While the record exists, R006
fires for every native expression the push will replace.

```
cadetta import ADAPTER FILE... --update [--write]
```

For bound paths whose observed value differs from the lock base,
`--update` prints a diff of `.cdt` literal edits; `--write` applies it.
A literal with a tolerance or fit is skipped and reported with the
observed value (edit by hand); a path with a citation is flagged ("value
no longer matches its cite"); `let` and derived paths are never touched
("derived; edit its sources"). After the edit, the next `sync`
classifies the locator as no-op. Contract 4.5 holds: nothing is written
without `--write`.

## Resolved questions

- OQ-7 (closed 2026-10-13, 5): adapters are pinned by `command` and
  `version` in `cadetta.toml`; `initialize` must match, else R008.
- OQ-8 (closed 2026-10-13, 2.2): attach or start hidden, refuse dirty
  documents (R009), save modified documents after a matching read-back.
