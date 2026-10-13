# cadetta-freecad

The FreeCAD reference adapter for cadetta (`docs/spec/adapters.md`). It is
an external adapter speaking JSON-RPC 2.0 on stdio, so stdout carries
protocol messages only and every log line goes to stderr
(`cadetta.logging.use_protocol_logging`).

Scaffold only: `cadetta-freecad --version` works; the protocol methods
are planned in the FreeCAD adapter epic.
