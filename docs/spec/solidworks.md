# SOLIDWORKS

SOLIDWORKS is the primary CAD target. This document specifies its
adapter and add-in and the workflow they support; the contract they
meet is adapters.md, the numbers they move are ir.md, and the geometry
they verify is geometry.md 6.

The goal is that an engineer says each thing once. Values, tolerances,
datums and part identities live in `.cdt`; shapes, sketches and mates
live in SOLIDWORKS; the link between them is made by selecting a native
item and picking a path, and is stored where SOLIDWORKS already stores
links (an equation, a feature name, a component's reference property),
so neither side restates the other.

## 1. Shape

One C# solution in `adapters/solidworks/`:

| Project | Target | Contents | Built in CI |
|---|---|---|---|
| `Cadetta.SolidWorks.Core` | netstandard2.0 | protocol types generated from `docs/schemas/adapter-protocol.json`; value and unit formatting at the declared precision; transform arithmetic; tolerance mapping (adapters.md 3.6); the `ISwSession` interface every SOLIDWORKS call goes through | yes |
| `Cadetta.SolidWorks.Com` | net48 | `ISwSession` over the SOLIDWORKS COM API; the only project that references the interop assemblies | no |
| `cadetta-solidworks` | net48 console | the protocol executable (adapters.md 5): `read` and `drive`, stdio JSON-RPC | yes, against a fake session |
| `Cadetta.SolidWorks.AddIn` | net48 COM add-in | the task pane (section 3) | no |
| `Cadetta.SolidWorks.Tests` | net8.0 | Core tests and transcript replay (section 6) | yes |

- The interop assemblies come from the local SOLIDWORKS install
  (`SOLIDWORKS_API_DIR`, default the install's `api/redist`); they are
  never committed. CI builds every project that does not reference
  them.
- Logging follows the house rule: stdout of `cadetta-solidworks` is
  protocol only, and logs go to stderr, filtered by `CADETTA_LOG`, in
  the format of the Rust `tracing` output. The add-in logs to
  `%LOCALAPPDATA%\cadetta\addin.log` with the same filter.
- `initialize` reports `capabilities = ["read", "drive"]` and
  `features = ["frames", "instances", "create_frames", "tolerances",
  "rename_globals"]` (adapters.md 5). `drive` attaches to a running
  SOLIDWORKS if there is one, else starts one hidden (adapters.md 2.2).

## 2. Workflow

### 2.1 A new part, design first

```
model Bracket {
  thickness = 6mm [+-0.1mm]
  datum base = origin
  datum top  = base.offset(z = thickness)
  holes: [Point2 on top; 4] = rect_pattern(2, 2, 40mm, 30mm) [position 0.2mm]
}
```

1. Open `Bracket.SLDPRT`. The Cadetta task pane lists `Bracket`'s
   paths, each with its value, tolerance, citation and doc comment.
2. Model as usual. Select the extrude depth dimension; the pane narrows
   to `Length` paths, nearest value first, and the engineer picks
   `Bracket.thickness`. The add-in creates the global `Bracket__thickness`
   if it is missing, sets the dimension's equation to
   `= "Bracket__thickness"` and writes `+-0.1mm` as its tolerance.
   Nothing is written to the bindings file: the equation is the binding
   (adapters.md 3.5).
3. "Create datum frames" creates the coordinate systems `Bracket__base`
   and `Bracket__top` from the composed transforms (the same thing
   `sync` does under `datums = "create"`). Sketch the holes on the face
   or plane at `top`, dimensioned from its origin; R007 reports a
   sketch that is not on the datum's plane.
4. Select the linear pattern's spacing dimensions and instance counts
   and link them to `holes.pitch_x`, `holes.pitch_y`, `holes.count_x`,
   `holes.count_y`.
5. `cadetta sync` (or the pane's Sync button) pushes values and
   tolerances, rebuilds, reads back, compares frames and records the
   lock.

### 2.2 An assembly

```
assembly Mount {
  part rail: Rail
  part bracket: Bracket
  place rail.origin at origin
  place bracket.base at rail.site
}
```

1. Insert the components. Select each and "Assign part": the add-in
   sets the component's reference property to the part's instance path
   (`rail`, `bracket`). That property is the binding (adapters.md 3.5).
   "Assign array" assigns `feet[0]` ... `feet[N-1]` to the selected
   instances by matching each instance's transform to the nearest
   composed instance frame; it refuses unless every match is unique
   within the adapter's precision, and never assigns by tree order.
2. Mate as you like. The recommended mate is one coincident mate between
   coordinate systems (`Bracket__base` to `Rail__site`): with the
   pattern-datum idiom every number lives in a datum, so no mate holds
   one (geometry.md 4, 6).
3. `sync` compares each component's transform with its composed
   placement; a part that sits elsewhere is R001 on
   `Mount/bracket.place`. A placement with ops can instead have its
   offset bound to a mate distance dimension ("Link dimension" on the
   mate's dimension).

### 2.3 Adopting an existing design

`cadetta import solidworks Part.SLDPRT` proposes members for globals
and dimensions, datums for coordinate systems and placements for
component instances (adapters.md 7). After `--accept`, the pane links
whatever the proposal skipped.

### 2.4 Without the add-in or COM

The `sw-equations` emitter writes a linked equations file
(emitters/sw-equations.md). With `[[adapter]] bind = "scope"` it holds a
global for every in-scope path, so in SOLIDWORKS the engineer links the
file once (Equations > Link to external file) and types `=` in a
dimension's Modify box to pick a global from the list. Those links are
native links, so switching to the `solidworks` adapter later needs no
rebinding.

## 3. The add-in

The add-in is a client of the `cadetta` command line. It never computes
a created name, evaluates a design, edits `.cdt` or edits a bindings
file itself; the names and the file format have one implementation, in
Rust.

- Project: the nearest `cadetta.toml` above the active document's
  folder. Executable: the add-in setting, default `cadetta` on `PATH`.
  Adapter: the first `[[adapter]]` with `name = "solidworks"`, or the
  one chosen in the pane.
- Reads: `cadetta names solidworks --format json` (config-cli.md 3)
  for paths, kinds, values, tolerances, citations, docs, created names
  and binding state; `cadetta sync --dry-run --format json` for drift
  badges.
- Writes to SOLIDWORKS documents only: globals, dimension equations,
  dimension tolerances, coordinate systems and component reference
  properties. A binding the native document cannot hold (a legacy global
  name, `value` mode, `for`, `config`, `sign`) goes through `cadetta
  bind` (config-cli.md 3).
- Commands:

  | Command | Selection | Effect |
  |---|---|---|
  | Link dimension | a dimension | equation `= "NAME"` to the chosen path's created global (created if missing), tolerance per adapters.md 3.6 |
  | Bind as value | a dimension or global | `cadetta bind PATH --adapter ID --locator ... --mode value` |
  | Unlink | a linked dimension | replaces the equation with its current value |
  | Assign part | a component | sets its reference property to a part instance path |
  | Assign array | several components | assigns indices by transform (2.2) |
  | Create datum frames | none (active document) | creates or updates the coordinate systems of the document's datums |
  | Show path | any bound item | path, value, tolerance, citation, `cadetta why` |
  | Sync, Dry run | none | runs `cadetta sync [--dry-run]` and shows the report |

- The pane refuses commands on a document that a running `sync` has
  open (adapters.md 2.2), and `sync` treats a document the add-in left
  dirty as R009, as for any dirty document.

## 4. Native mapping

| Cadetta | SOLIDWORKS |
|---|---|
| `global` locator | equation manager global variable, written `"NAME" = 6mm` |
| `dimension` locator | `D1@Boss-Extrude1@Bracket.SLDPRT`; `ref` mode writes the equation `"D1@Boss-Extrude1" = "NAME"` |
| dimension tolerance (adapters.md 3.6) | the dimension's tolerance: symmetric, bilateral, limit, fit with hole or shaft class, or none |
| `property` locator | custom property, configuration-specific when `config` is set |
| `frame` locator | a coordinate system feature; an explicit locator may name a plane feature, which fixes only origin and z, so only those are compared |
| `instance` locator | a component, named `Bracket-1` (`Sub-1/Bracket-1` inside a subassembly); implicit through the reference property |
| pattern count | the instance count of a linear, circular or sketch-driven pattern |
| transform | a coordinate system's transform in its document, a component's transform in its assembly, converted from metres to the native unit |

- Created names (adapters.md 3.1) use only ASCII letters, digits and
  `_`, which SOLIDWORKS accepts as global and feature names.
- The API surface the implementation is expected to use: the equation
  manager (globals, equations), `IDimension` and `IDimensionTolerance`,
  custom property managers, coordinate system creation from numeric
  values and the extension's coordinate system transform query,
  `IComponent2` transform and reference property, and `ForceRebuild3`
  with the rebuild error list. Each is confirmed on a licensed machine
  by the release smoke (section 6); where a SOLIDWORKS version lacks
  one, `initialize` omits the feature that needs it.

## 5. Failure mapping

| SOLIDWORKS condition | Code |
|---|---|
| not installed, unsupported version, interop load failure | R008 |
| document read-only, checked out elsewhere, dirty | R009 |
| driven (reference) dimension, suppressed feature, equation conflict | R010 |
| rebuild errors after push | R004, then the snapshot restore of adapters.md 2.3 |
| bound item missing | R003 |

### 5.1 Save

`Cadetta.SolidWorks.Save.SaveService` is the `save` step: with
`save = "after-sync"` it writes only documents the adapter modified whose
rebuild did not fail and records a `blake3:` digest of the saved bytes
(`Blake3.Digest`); with `save = "never"`, or when any locator `failed`, it
writes nothing and the documents stay dirty. `SwComSaveTarget` saves a
`ModelDoc2` silently through `Save3`.

### 5.2 Push

`Cadetta.SolidWorks.Push.PushService` is the `push` step over the
`IPushDocument` seam (`SwComPushDocument` is the COM implementation;
native units are mm and deg).

- Kind inference: `ChangedParam` carries no kind, so the locator prefix
  `global:`, `dimension:`, `property:` or `frame:` is honoured first;
  mode `frame` is a coordinate system; mode `ref` or an `@` in the name
  is a dimension; otherwise an existing global, then an existing custom
  property, else a new global. A new custom property therefore needs the
  `property:` prefix until the wire gains a `kind` field.
- Ref mode writes the target global once (ref_to, else the first global
  in the batch bound to the same path, else the mangled path) and links
  the dimension to it through an equation. A later value push to a
  linked dimension drops the link and warns R006.
- Only locators whose value, link or tolerance changed are written;
  unchanged ones return `skipped`. A push with no change does not
  rebuild.
- Guards: a dirty (user-edited) or read-only document rejects every
  locator with R009. An equation-linked dimension is pushable; a true
  reference dimension, a suppressed feature or an equation conflict is
  R010. A value that does not fit the precision is Q004 (rejected).
- Tolerances use `IDimensionTolerance` (`symmetric`, `bilateral`,
  `limits`, `fit`); frames use
  `CreateCoordinateSystemUsingNumericalValues` (rotation transposed in
  slots 0-8, translation in 9-11) and are created only with
  `datums = create`, otherwise R003 skipped.
- After writing, one `ForceRebuild3`; a rebuild error is R004 (failed)
  and every write is undone from the recorded undo list.

## 6. Testing

- `Cadetta.SolidWorks.Tests` covers Core on Linux and Windows.
- Transcripts: a recording `ISwSession` wrapper captures every call and
  result during a real session as JSON lines; the fake session replays
  them, so the protocol executable's `read`, `push`, `regen` and `save`
  paths run in CI with byte-exact protocol output.
  Implemented in `Cadetta.SolidWorks.Session.Recording`: `RecordingSwSession`
  writes one canonical JSON line per call (`seq`, `call`, `args`, `result`,
  optional `error`); `ReplaySwSession` answers from those lines in order and
  throws `TranscriptMismatchException` on any divergence. Fixtures live in
  `adapters/solidworks/tests/Cadetta.SolidWorks.Tests/Transcripts/`.
- Release smoke, on a licensed machine, recorded as transcripts for the
  next CI run: open the pilot r3-05 part; link a dimension, a pattern
  count and a tolerance through the add-in; create datum frames; assign
  parts in a two-part assembly; `sync`, edit a value in SOLIDWORKS,
  `sync` again (R001 cad moved), `import --update`, `sync` (no-op).
