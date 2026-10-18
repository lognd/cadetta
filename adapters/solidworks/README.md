# cadetta-solidworks

The SOLIDWORKS adapter (docs/spec/solidworks.md). One C# solution:

| Project | Target | Built in CI |
|---|---|---|
| `Cadetta.SolidWorks.Core` | netstandard2.0 | yes |
| `Cadetta.SolidWorks.Com` | net48 | no (needs the interop assemblies) |
| `cadetta-solidworks` | net48 console | yes, with `-p:CadettaNoCom=true`, against the fake session |
| `Cadetta.SolidWorks.AddIn` | net48 | no (needs the interop assemblies) |
| `Cadetta.SolidWorks.Tests` | net8.0 | yes |

Only `Com` and `AddIn` reference the SOLIDWORKS interop assemblies, via
`Interop.props`, from `SOLIDWORKS_API_DIR` (default
`C:\Program Files\SOLIDWORKS Corp\SOLIDWORKS\api\redist`). They are never
committed.

## Shipped in 0.1.0

The protocol executable (`cadetta-solidworks`), the add-in (`Cadetta` menu
and task pane: Link, Unlink, Sync preview, Sync, Why, Assign array; see
`src/Cadetta.SolidWorks.AddIn/README.md`), the snapshot and atomic restore,
frame and coordinate-binding support, and twelve recorded transcript
fixtures (`tests/Cadetta.SolidWorks.Tests/Transcripts/`, replayed in CI
against fakes). It was live-tested against SOLIDWORKS 2025 (revision 33.3.0)
on nova: `SMOKE.md` has the release smoke results and `evidence/` the logs
of each live run (`evidence/<ticket>-live/`). `API-SURFACE.md` records the
probed API results and `DEVELOPING.md` the bootstrap.

## Build and test

```
dotnet build src/Cadetta.SolidWorks.Core
dotnet build src/cadetta-solidworks -p:CadettaNoCom=true
dotnet test  src/Cadetta.SolidWorks.Tests
```

Do not build the whole `.sln` without SOLIDWORKS: `Com` and `AddIn` need
the interop assemblies.

## Protocol types

`src/Cadetta.SolidWorks.Core/Protocol/Protocol.g.cs` is generated from
`docs/schemas/adapter-protocol.json`:

```
python3 tools/gen_protocol.py          # regenerate
python3 tools/gen_protocol.py --check  # CI: fail if stale
```

## Running

`cadetta-solidworks [--fake] [--version]` speaks newline-delimited
JSON-RPC 2.0 on stdio. Stdout is protocol only; logs go to stderr,
filtered by `CADETTA_LOG` (`error|warn|info|debug|trace`, optionally
`cadetta_solidworks=LEVEL`). `--fake` (or `CADETTA_SW_FAKE=1`) uses the
in-memory session. Methods: `initialize`, `shutdown`, `read`, `push`, `regen`,
`open`, `save` and `close`. `read` handles one file per call through
`ReadService`, and so through `ReadCache` when the server was built with one
(several files or a session source answer -32602, a file that cannot be
opened -32000; a server built without a `ReadService` answers -32601).
`push` opens the file through an `IPushHost` (the COM host reuses an open
copy, otherwise opens it silently and closes it afterwards), runs
`PushService`, then the save step in the default `after_sync` mode, and
returns one status per locator; a file that cannot be opened answers -32000,
a missing `file` -32602 and a server without a host -32601. `regen` rebuilds
each target and reports R004 per failed feature (a server without a regen host
answers -32601). `open` returns a session handle only: no document is held,
each push opens, saves and closes its own file, so `save` and `close` have
nothing left to do. Without `--fake` the executable wires the COM read
service (with `ReadCache`), push host and regen host; the fake session has
none of them. Any other method answers -32601.

## Session lifetime

Without `--fake`, the Com build connects through `SwConnection<T>`
(`Core/Session`): it attaches to a running SOLIDWORKS if there is one,
otherwise starts one hidden through `SwComHost`. On exit it quits the
instance only if it started it (adapters.md 2.2).

## Busy detection (R009)

Before any push, `BusyGuard.Check` (`Core/Busy`) asks an `IBusyProbe`
whether a modal dialog is open or a command is active; either refuses
the push with R009 and nothing is written (adapters.md 2.3). The Com
build's `SwComBusyProbe` uses `CommandInProgress` and a disabled main
window. `DocumentGate` (`Core/Commands`) applies it, with the read-only and
sync-lock checks, to the add-in's link commands. The `push` RPC refuses a
read-only document or one holding unsaved edits (R009) in `PushService`, but
does not call `BusyGuard`.

## Transient COM faults

A `read` right after attaching can hit `RPC_E_SERVERFAULT`
(0x80010105) or a busy rejection (0x80010001, 0x8001010A) while
SOLIDWORKS is still settling. `TransientRetry.Run` (`Core/Read`) retries
those up to 4 attempts with linear backoff (500 ms steps), logging each
retry at warn; other errors and the final failure propagate.

## Probing the API

`cadetta-solidworks --probe-api <scratch-dir>` exercises the SOLIDWORKS
calls of the spec against a running instance (manual, not in CI). See
`DEVELOPING.md` for bootstrap and `API-SURFACE.md` for results.

## Snapshot and atomic restore (R004)

`AtomicPush.Run` (`Core/Restore`) snapshots every locator a push will
change through an `IRestorableDocument`, writes the new values and
rebuilds. When the rebuild fails it sets the snapshotted values back and
rebuilds again, returning an R004 diagnostic that says whether the
restore succeeded (adapters.md 2.3). The Com build's
`SwComRestorableDocument` maps locators to dimension names and uses
`SetSystemValue3` and `ForceRebuild3`.

## Frames, coordinate bindings and R007

`Core/Frames` carries the frame side of the adapter protocol (solidworks.md 4,
adapters.md 3.4, geometry.md 6):

- `FrameReader` reads each bound frame locator (a coordinate system or a
  reference plane feature name) into `frames[]`, translated to the native unit.
  A plane feature compares only its origin and z axis.
- `FrameWriter` serves `datums = create`: it creates the coordinate system from
  numeric values, skips it when it already matches within `0.5 * 10^-precision`,
  and updates it otherwise. A plane feature locator cannot be driven and fails.
- `CoordinateBinder` pushes a datum-plane sketch coordinate through its bound
  dimension as `sign * value * unit`; sign must be 1 or -1.
- `SketchObserver` produces the `sketches[]` facts (`on_datum_plane`,
  `measured_from`) from which the host raises R007. The push still proceeds
  for an unclean sketch, with a warning, since R007 is only a warning.

`Com/Frames` is the SOLIDWORKS binding. Confirmed on SOLIDWORKS 2025 (revision
33.3.0, nova), evidence in `tests/live/frames-nova-live.log`:

- The API has no in-place coordinate system update, so an update deletes and
  recreates the feature under the same name (references to it are lost). The
  delete must select the feature itself (`IFeature.Select2`) after a rebuild;
  `SelectByID2(name, "COORDSYS")` selects but then `DeleteSelection2` fails.
- Euler angles are `R = Rx * Ry * Rz` (x, y, z passed in that order), checked
  through create and read-back on 11 single-axis and composite cases. The
  read-back comparison (R001) remains as the guard.
- `measured_from` is read from the dimension's attached entities: `sketch
  origin` when one is the origin point at (0,0,0), else `other`; `unknown`
  only when the dimension cannot be selected. The sketch is the segment after
  the first `@` of the full name (`D1@Sketch2@Part1.Part`).
