# Cadetta.SolidWorks.AddIn

The SOLIDWORKS COM add-in (net48). It is a pure client of the `cadetta` CLI:
it never computes names and never edits `.cdt` (docs/spec/solidworks.md 3).

## What it does

- On document open and on active-document change it finds the nearest
  `cadetta.toml` above the document, picks the first `[[adapter]]` with
  `name = "solidworks"` (its `id` defaults to the name), and runs
  `cadetta names <adapter-id> --format json` with the project root as the
  working directory.
- The task pane lists the document component's paths with value (`emit`),
  tolerance, citation and doc. The component is the file stem matching the
  first segment of the path after the last `::`.
- When a dimension is selected, the pane keeps only `param` rows of the same
  quantity kind (length or angle) and sorts them by nearness to the value.

## Layout

| Folder | Interop? | Purpose |
|---|---|---|
| `Client/` | no | project discovery, process runner, `names` JSON model |
| `Pane/` | no | quantity parsing, path filtering, `PaneModel` |
| `Logging/` | no | `%LOCALAPPDATA%\cadetta\addin.log`, filtered by `CADETTA_LOG` |
| root `.cs` | yes | `CadettaAddIn` (ISwAddin), `TaskPaneControl`, `SwSelection` |

The interop-free folders are linked into
`tests/Cadetta.SolidWorks.Tests/AddIn`, which runs on plain `dotnet test`.

## Configuration

The executable is `CADETTA_EXE`, else `HKCU\Software\Cadetta\SolidWorks\Executable`,
else `cadetta` on `PATH`. Log level comes from `CADETTA_LOG` (for example `debug`).

## Registration

`regasm /codebase Cadetta.SolidWorks.AddIn.dll` (elevated) writes the
`HKLM\SOFTWARE\SolidWorks\Addins` key; `regasm /u` removes it.

## Commands (link, assign, datums, sync)

- `Core/Commands/LinkCommands` links or unlinks a dimension to a created global (`"D1@Feature" = "NAME"`); the name comes from `cadetta names`, never computed here. Driven, suppressed, read-only or conflicting dimensions are refused (R010).
- `Core/Commands/DocumentGate` refuses any write while a sync is running, the document is read-only, or the session is busy (R009).
- `Core/Commands/FrameMatcher` assigns an array instance by the unique nearest pose, and refuses when empty, too far or tied.
- `AddIn/Commands/CadettaCommands` drives `cadetta bind`, `bind --remove`, `sync [--dry-run] [--only GLOB]` and `why`; it never edits bindings or the .cdt itself. Create datum frames is `sync --only <datum paths>` and needs `datums = "create"`.

Not exercised live: the COM wiring (menu items, selection reading) is not part of this change; the logic above is unit-tested through `tests/Cadetta.SolidWorks.Tests/Commands`. No CLI verb yet reports composed datum or instance frames, so assign array needs a `cadetta frames` surface.
