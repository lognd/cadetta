# cadetta-freecad

The FreeCAD reference adapter for cadetta (`docs/spec/adapters.md`). It is
an external adapter speaking JSON-RPC 2.0 on stdio, so stdout carries
protocol messages only and every log line goes to stderr
(`cadetta.logging.use_protocol_logging`).

`FreeCadAdapter` subclasses `cadetta.adapter.Adapter` (bindings.md 3.4),
so the JSON-RPC loop is the base class's. `initialize` reports
capabilities emit, read and drive, native unit mm and precision 6. `emit`
writes bound values as Spreadsheet aliases (`emit.py`, adapters.md 6.1);
`read` imports Spreadsheet aliases and Sketcher dimensions of FCStd files
into observed IR (`read.py`, adapters.md 6.2): one `<pkg>::Observed`
component, imported provenance, driven dimensions and long binary64
values `exact: false` with a reason (Q006); sessions are unsupported.
`open`, `push`, `regen`, `save` and `close` drive (`drive.py`, adapters.md
6.3): sessions attach to or hide-open FCStd files, refuse dirty or read-only
ones (R009), push only changed locators, report rebuild failures as R004
and restore the snapshot, and save only after a good rebuild. FreeCAD is reached
through the `FreeCadBackend` seam (`backend.py`) so the protocol is
testable without FreeCAD. `cadetta-freecad --version` prints the version.

`tests/conformance/session_driver.py` drives the real adapter's `open` and `close`
over a fake tool for the attach and close-only-opened conformance fixtures
(`tests/conformance/adapters/session-*`), which the Rust corpus runner invokes
through `uv run`.

The live round trip has been run against FreeCAD 1.1.4 `freecadcmd` on nova
(Windows): see `evidence/9ksan8k-live/` for the transcript and the recipe.
