# Developing the SOLIDWORKS adapter on Windows

Bootstrap verified on host `nova` (Windows ARM64, SOLIDWORKS 2025).

| Item | Version / value |
|---|---|
| .NET SDK | 9.0.301 (builds net48, netstandard2.0, net8.0) |
| SOLIDWORKS | 2025, interop version 33.3.0.92 |
| `SOLIDWORKS_API_DIR` | `C:\Program Files\SOLIDWORKS Corp\SOLIDWORKS\api\redist` (default) |
| Host CPU | ARM64; SOLIDWORKS is registered x64 and runs emulated |

## Steps

1. Install the .NET SDK (9.x) and SOLIDWORKS 2025.
2. Optionally set `SOLIDWORKS_API_DIR` to the folder holding
   `SolidWorks.Interop.sldworks.dll` and `SolidWorks.Interop.swconst.dll`.
   They are never committed.
3. Clone the repo, then from `adapters/solidworks`:
   - `dotnet test src\Cadetta.SolidWorks.Tests` (28 tests pass on nova)
   - `dotnet test tests\Cadetta.SolidWorks.Tests\AddIn` (add-in logic; no
     SOLIDWORKS or net48 needed; also run by CI)
   - `dotnet test tests\Cadetta.SolidWorks.Tests\Commands` (add-in command
     logic; also run by CI)
   - `dotnet build src\cadetta-solidworks -c Release` (builds Com + exe)
4. The exe resolves `SolidWorks.Interop.*` from `SOLIDWORKS_API_DIR` at run
   time (`InteropResolver`), because the references are `Private=false`.

## API probe

`cadetta-solidworks.exe --probe-api <scratch-dir>` runs
`SwComApiProbe` (Com project, behind `ISwApiProbe`; `FakeApiProbe` in Core)
and prints a markdown table (API | Call | Result | Detail). It is a manual
tool: it is not run in CI and the exe returns 2 without COM. Results go in
`API-SURFACE.md`.

## Read to observed IR

`cadetta-solidworks.exe --read-doc <file> <out.json>` opens the document
through `SwComSession.ReadObserved` (`SwComDocumentReader`), builds the
observed IR via `ReadService` and writes the serialized `ReadResult`.
`--make-read-fixture <part>` authors `Fix.SLDPRT` (symmetric tolerance on
`D1@Boss-Extrude1`) and `FixAsm.SLDASM` (one component with reference
`REF_A`) beside the part. Both need COM and run in the interactive
session; they return 2 otherwise. Core logic (`ObservedIrBuilder`,
`ReadService`, `FakeNativeDocumentReader`) is covered by unit tests.

## ARM64 notes

- The net48 AnyCPU exe runs on nova and loads the interop assemblies.
- Starting SOLIDWORKS over plain ssh fails: `CreateInstance` of the
  SldWorks CLSID returns `CO_E_SERVER_EXEC_FAILURE` (0x80080005) from the
  non-interactive session 0. SOLIDWORKS needs the interactive console
  session (session 1). Run the probe from a terminal in that session, or
  start SLDWORKS.exe there first so the exe attaches to the running one.
  With SLDWORKS.exe already running in session 1, an ssh (session 0) run
  still cannot attach: `GetActiveObject` fails with MK_E_UNAVAILABLE
  (0x800401E3) because the running object table is per logon session, and
  the fallback start then spawns a second, hung SLDWORKS in session 0.
  Scheduled tasks created with `/IT` stayed `Queued` on nova.
- Fixed while bootstrapping: `SwComSession`'s `ISldWorks` constructor is now
  `internal` so `Program.cs` does not need the interop reference (CS0012).

## Running the probe from ssh (session 1 technique)

The running object table is per logon session, so the probe must run in the
console session where SOLIDWORKS is open. `tools/probe-api.ps1` registers an
Interactive-principal scheduled task (session 1), starts it, waits for it
to finish, prints the result and removes the task. The command line is
non-interactive (no `more`, `pause` or `cmd /k`), so no window lingers;
output goes to `<scratch>\probe-result.md` and `probe.err`.
Before each run check for leftover windows with
`Get-Process cmd | ? SessionId -eq 1` and run one probe at a time.

## Findings

- Equations silently ignore invalid dimension values; use a shell too thick
  for the part to provoke a genuine rebuild error.
- Look features up by type name, never by localized name.
- Results are recorded in `API-SURFACE.md`.

## Regen (rebuild-error mapping)

The `regen` RPC takes file-path targets, opens each through `IRegenHost`
(`SwComRegenHost` in the Com project, fakes in Core tests), forces a
rebuild and walks the feature tree with `IFeature.GetErrorCode2`.
`RegenService` maps each failing feature to an `R004` diagnostic that names
the feature. A document with unsaved user changes, or opened read-only, is
refused with `R009` and never rebuilt (OQ-8). A regen failure during a push
is handled by `PushService`/`AtomicPush` (R004 plus atomic restore,
adapters.md 2.3). Live check: build a shelled part, link the shell
thickness to a global, set it too large, save, and regen (see the
`hxvd96n-nova-live` evidence on the ticket).

## Live check of the add-in (nova)

- Build with `dotnet build -c Release -o <new dir> -p:Version=<n>`; a managed
  dll loaded in SOLIDWORKS stays locked, so use a new dir and a new version
  for every attempt, then `regasm /codebase` each (64-bit
  `Framework64\v4.0.30319\regasm.exe`). Unregister stale builds with
  `regasm /u`.
- COM needs the interactive console session: drive it from a net48 tool run
  as an Interactive scheduled task (see "Running the probe from ssh").
- Log open documents before and after every step and close what you opened
  without saving. Do not kill SLDWORKS; nova may show a second, stray
  SLDWORKS with an empty title (the real one is the ROT-attached process).
- Set `CADETTA_EXE` or HKCU `Software\Cadetta\SolidWorks\Executable`, and
  `CADETTA_LOG=debug` or HKCU `...\Log` to read `%LOCALAPPDATA%\cadetta\addin.log`.
- Status (~EXF7N54): `ISldWorks.LoadAddIn(path)` is not an activation
  path: it returns 0 for any existing file (even kernel32.dll) and 3 for a
  missing one, and never loads a registered COM add-in. Cadetta is listed
  under "Other Add-ins" in Tools > Add-Ins (registration works) but
  unticked. Registration now writes HKLM Addins default 1 and HKCU
  AddInsStartup 1, so SOLIDWORKS loads it at the next startup (re-run
  `regasm /codebase` with the framework64 regasm, then restart SOLIDWORKS)
  or when the Add-Ins box is ticked. `ConnectToSW` now logs at warn level so
  a connect shows in addin.log by default. Evidence: `evidence/exf-live/`.
- Release smoke (~NEHY9X7): the add-in connects, and the drive protocol
  (`open`, `read`, `push`, `regen`, `save`, `close`) round-trips on a
  stand-in part; see `SMOKE.md` and `evidence/nehy-live/`. Gotchas: the ctl
  tool's `setdim` takes metres; `D1@Shell1` does not move through the API;
  a CAD tolerance not present in the design makes read-back differ from the
  pushed literal, so clear it before a sync test.

## Live drivers must be C# (nova)

PowerShell on nova cannot cast the SOLIDWORKS COM object to the interop
`ISldWorks`. Write live drivers as net48 exes that call
`InteropResolver.Install`, build them on nova and run them through the
session-1 scheduled task. See `tools/pf7x/` and `tools/w5mb/` (build, edit a dimension, list/close docs). A loaded managed dll stays
locked, so restart SOLIDWORKS when swapping the add-in build.
