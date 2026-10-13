+++
id = "01M4SPSVXQFVQN3519BD55Z3DK"
version = "0.1.0"
goal = "Ship the .cdt language, exact quantity system, cadetta.ir/1 IR and JSON schema, lint catalog, built-in emitters (JSON, CSV, SOLIDWORKS equations), adapter protocol, and a FreeCAD reference adapter that reads and drives headless in CI"
state = "open"
epics = ["01M4SPWFRR9DAEM60DX796GVPN", "01M4SPWFZP4041AN6YQE08X2AN", "01M4SPWG5A0FJNPKRPDDCJQZN8", "01M4SPWGB2729B40CQE81MHM2Z", "01M4SPWGV64X8H1TN1YBR6K5CH", "01M4SPWH0ZE4G7KXRDA5M71GPS", "01M4SPWH74K9QQS72Z9YMTPD72"]
created = "2026-10-13T04:02:19Z"
updated = "2026-10-13T04:03:47Z"

[[criteria]]
text = "cadetta check lints a .cdt project and gates on fail_on"
bound = false

[[criteria]]
text = "json, csv and sw-equations emitters produce deterministic golden output"
bound = false

[[criteria]]
text = "FreeCAD adapter round-trips emit, read and drive headless in CI"
bound = false

[[criteria]]
text = "cadetta.ir/1 JSON schema committed under docs/schemas/"
bound = false
+++
