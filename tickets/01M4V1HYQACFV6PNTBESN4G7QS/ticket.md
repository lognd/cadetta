+++
id = "01M4V1HYQACFV6PNTBESN4G7QS"
title = "W003 not raised for waive or allow naming a require diagnostic"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:29:29Z"
updated = "2026-10-15T02:51:47Z"
scope = ["crates/cadetta-cli/src/**", "docs/spec/lints.md", "tests/conformance/known-gaps.toml", "changelog.d/01M4V1HYQACFV6PNTBESN4G7QS.fixed.md"]

[[acceptance]]
text = "allow on a require raises W003 and C001 stays an error; allow on a check suppresses it"
bound = true

[[acceptance]]
text = "waive naming a require diagnostic raises W003 and does not suppress it"
bound = false
+++

obligations.md 4 / lints.md 2.3: waive C001 on a require suppresses it (exit 0) and allow(C001) on a require gives no W003; spec: require is never suppressed and naming it is W003. found while working ~P3E2FR7
