+++
id = "01M4VQXM5M08SGAEJQTQ6AKDVQ"
title = "Conformance runner: executor for binding protocol entries"
type = "task"
category = "todo"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:00:20Z"
updated = "2026-10-13T23:00:20Z"
scope = ["crates/cadetta-cli/tests/**"]

[[acceptance]]
text = "Every kind=binding entry=protocol fixture executes and is compared; their known-gaps.toml entries are removed."
bound = false
+++

found while working ~6BCVN48: kind=binding entry=protocol (adapters 24, emitters 3) has no executor; drive the adapter host JSON-RPC against the test adapter and compare with expect.json.
