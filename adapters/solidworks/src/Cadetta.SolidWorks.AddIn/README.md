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
