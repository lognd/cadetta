+++
id = "01M4SV2TCZM23R46EM75RYSA3B"
title = "SOLIDWORKS adapter (adapters/solidworks)"
type = "epic"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T05:17:07Z"
updated = "2026-10-13T05:17:07Z"
+++

Outcome: the primary 0.1.0 CAD target. Python on pywin32 COM, built on cadetta.adapter.Adapter (bindings.md 3.4); emit, read and drive for equation manager globals, dimensions and custom properties (adapters.md 6). CI cannot run SOLIDWORKS: tests replay recorded COM transcripts; releases need a manual smoke on a licensed machine. Windows only.
