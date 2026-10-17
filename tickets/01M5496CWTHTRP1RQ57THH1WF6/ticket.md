+++
id = "01M5496CWTHTRP1RQ57THH1WF6"
title = "Wire protocol: add locator kind to ChangedParam so new custom properties need no property: prefix"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T06:36:08Z"
updated = "2026-10-17T06:58:09Z"
scope = ["docs/spec/adapters.md", "adapters/solidworks/src/Cadetta.SolidWorks.Core/Protocol/**", "docs/schemas/adapter-protocol.json", "crates/cadetta-adapter/src/rpc/types.rs", "crates/cadetta-adapter/src/rpc/client.rs", "crates/cadetta-adapter/src/session.rs", "crates/cadetta-cli/src/cmd/sync.rs"]
+++

found while working ~DKCF816: kind is inferred from name/mode (see solidworks.md 5.2); a new property needs the property: prefix.
