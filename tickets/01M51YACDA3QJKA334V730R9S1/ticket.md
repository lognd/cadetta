+++
id = "01M51YACDA3QJKA334V730R9S1"
title = "universe test fit_obligation_is_listed_signed_off_and_stales_on_edit fails on main"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-16T08:47:36Z"
updated = "2026-10-16T08:48:06Z"
idempotency_key = "universe-fit-obligation-test"
scope = ["crates/cadetta-cli/src/universe.rs"]

[[acceptance]]
text = "The named test passes on main"
bound = false
+++

Found by ~X50QNG4: universe::tests::fit_obligation_is_listed_signed_off_and_stales_on_edit fails on main independent of that change. Diagnose and fix.
