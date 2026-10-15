+++
id = "01M4YQFTG7GPRHE6E1PH29EAMF"
title = "Q004 beyond-precision error not raised (q004, contract-q004, value-q004 fixtures)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:50:31Z"
updated = "2026-10-15T02:54:22Z"
idempotency_key = "mhtx-q004-split"
labels = ["creates:changelog.d/01M4YQFTG7GPRHE6E1PH29EAMF.fixed.md"]
scope = ["crates/cadetta-units/**", "crates/cadetta-sema/**", "crates/cadetta-emit*/**", "crates/cadetta-cli/src/cmd/emit.rs", "crates/cadetta-cli/tests/**", "tests/conformance/known-gaps.toml", "docs/spec/config-cli.md", "changelog.d/01M4YQFTG7GPRHE6E1PH29EAMF.fixed.md"]

[[acceptance]]
text = "sw-equations Q004 beyond-precision fixtures fire (error, path, exit 1, file withheld) and pass when representable"
bound = false

[[acceptance]]
text = "emit JSON diagnostics are cadetta.diag/1 records with severity"
bound = false
+++

split from ~MHTX343 (Q006 exact-zero/trig now verified). Q004 error for values not representable at adapter precision is not emitted by emitters/contract.
