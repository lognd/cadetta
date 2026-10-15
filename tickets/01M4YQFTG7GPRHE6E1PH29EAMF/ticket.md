+++
id = "01M4YQFTG7GPRHE6E1PH29EAMF"
title = "Q004 beyond-precision error not raised (q004, contract-q004, value-q004 fixtures)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:50:31Z"
updated = "2026-10-15T02:50:31Z"
idempotency_key = "mhtx-q004-split"
scope = ["crates/cadetta-units/**", "crates/cadetta-sema/**", "crates/cadetta-emit*/**"]
+++

split from ~MHTX343 (Q006 exact-zero/trig now verified). Q004 error for values not representable at adapter precision is not emitted by emitters/contract.
