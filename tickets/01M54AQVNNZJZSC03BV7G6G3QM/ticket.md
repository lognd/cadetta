+++
id = "01M54AQVNNZJZSC03BV7G6G3QM"
title = "Host: send the binding locator kind in push ChangedParam (cmd/sync.rs sets kind: None)"
type = "task"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T07:03:09Z"
updated = "2026-10-17T07:03:26Z"
scope = ["crates/cadetta-adapter/src/sync.rs", "crates/cadetta-adapter/src/atomic.rs", "crates/cadetta-cli/src/cmd/sync.rs"]
+++

found while working ~THH1WF6: the wire now has ChangedParam.kind but PushItem carries no kind, so the host still sends None and adapters infer it. Plumb the binding's LocatorKind through PushItem.
