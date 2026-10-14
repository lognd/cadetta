# cadetta-freecad

The FreeCAD reference adapter for cadetta (`docs/spec/adapters.md`). It is
an external adapter speaking JSON-RPC 2.0 on stdio, so stdout carries
protocol messages only and every log line goes to stderr
(`cadetta.logging.use_protocol_logging`).

`FreeCadAdapter` subclasses `cadetta.adapter.Adapter` (bindings.md 3.4),
so the JSON-RPC loop is the base class's. `initialize` reports
capabilities emit, read and drive, native unit mm and precision 6; the
other methods are planned in the FreeCAD adapter epic. FreeCAD is reached
through the `FreeCadBackend` seam (`backend.py`) so the protocol is
testable without FreeCAD. `cadetta-freecad --version` prints the version.
