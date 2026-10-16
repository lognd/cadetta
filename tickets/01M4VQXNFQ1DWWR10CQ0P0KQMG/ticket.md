+++
id = "01M4VQXNFQ1DWWR10CQ0P0KQMG"
title = "sw-equations emit details: native pins, was comments, mangle prefix, order, comment shapes, native unit suffix, N005/R013 mangle checks"
type = "bug"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T23:00:21Z"
updated = "2026-10-16T19:49:13Z"
scope = ["crates/cadetta-adapter/**", "crates/cadetta-cli/src/cmd/emit.rs", "crates/cadetta-sema/src/attrs.rs"]

[[acceptance]]
text = "The listed fixtures pass and their gap entries are removed."
bound = true

[[acceptance]]
text = "All 11 sw-equations emit fixtures listed under this ticket pass and their known-gaps entries are removed"
bound = false
+++

found while working ~6BCVN48: adapters/{native-pin-*,was-*,mangle-*,sw-eq-lines-topological,bind-values-not-expressions,bind-native-unit-default} and emitters/sw-equations/{comment-*,value-native-unit-suffix,naming-component-field,global-created-unit-from-locator} disagree with the spec (see known-gaps.toml entries under this ticket).
