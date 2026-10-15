+++
id = "01M4VQXM0V12H5JBPHWW796HD7"
title = "Conformance runner: executor for binding read entries (adapter host)"
type = "task"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4STAPYJBMRZBEY8HJH6F6XS"
reporter = "lognd"
created = "2026-10-13T23:00:20Z"
updated = "2026-10-15T08:02:57Z"
scope = ["crates/cadetta-cli/tests/**", "tests/conformance/README.md", "tests/conformance/known-gaps.toml", "changelog.d/01M4VQXM0V12H5JBPHWW796HD7.changed.md"]

[[acceptance]]
text = "Every kind=binding entry=read and emit-read fixture executes against the adapter host and its expect.json is compared; their known-gaps.toml entries are removed."
bound = false
+++

found while working ~6BCVN48: kind=binding entry=read (adapters 21, emitters 14, geometry 1 fixtures) has no executor in crates/cadetta-cli/tests/conformance/exec.rs; drive the test adapter (crates/cadetta-adapter/src/testing.rs) and compare with expect.json per tests/conformance/README.md.
