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
in-memory session. Implemented so far: `initialize` and `shutdown`; other
methods answer -32601 until their tickets land.

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
window. The guard is not yet wired into a `push` method (it does not
exist yet).

## Snapshot and atomic restore (R004)

`AtomicPush.Run` (`Core/Restore`) snapshots every locator a push will
change through an `IRestorableDocument`, writes the new values and
rebuilds. When the rebuild fails it sets the snapshotted values back and
rebuilds again, returning an R004 diagnostic that says whether the
restore succeeded (adapters.md 2.3). The Com build's
`SwComRestorableDocument` maps locators to dimension names and uses
`SetSystemValue3` and `ForceRebuild3`.
