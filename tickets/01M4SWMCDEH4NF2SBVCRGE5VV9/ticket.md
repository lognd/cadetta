+++
id = "01M4SWMCDEH4NF2SBVCRGE5VV9"
title = "Locator fields, [*] bindings, collisions R012 and R013"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T05:44:11Z"
updated = "2026-10-13T14:11:29Z"
persona = "design engineer"
capability = "bind every instance safely"
idempotency_key = "replan14:ad-loc"
labels = ["creates:crates/cadetta-adapter/src/locator.rs"]
scope = ["crates/cadetta-adapter/src/locator.rs", "crates/cadetta-adapter/src/bindings.rs", "crates/cadetta-adapter/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWRYRBTGD67WTG7D60E59"

[[acceptance]]
text = "Given a binding with kind, mode, ref_to, for, config, sign and component, when loaded, then each is validated (adapters.md 3; ir.md 3)"
bound = true

[[acceptance]]
text = "Given a [*] binding whose instances emit different values, when emitting, then R012 fires; given one native name from two paths, then R013 fires and the emit fails (adapters.md 3; lints.md 3 R)"
bound = true

[[acceptance]]
text = "Given the same native name in two namespaces, when emitting, then names are namespace-qualified (adapters.md 3)"
bound = false
+++
