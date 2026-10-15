+++
id = "01M4YNBWZ43RJ19ECMQ4280WEP"
title = "mmc/lmc: feed fit-side roles (RoleSource::FitSide) into material-side selection"
type = "task"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T02:13:25Z"
updated = "2026-10-15T02:36:14Z"
idempotency_key = "fitside-roles-mmc-lmc"
scope = ["crates/cadetta-sema/src/**", "docs/spec/quantities.md", "changelog.d/01M4YNBWZ43RJ19ECMQ4280WEP.*"]

[[acceptance]]
text = "mmc/lmc on a member whose role comes only from a fit side resolves the material side without E010"
bound = true
+++

Follow-up of ~7078TNV: RoleSource::FitSide is not yet consulted by mmc/lmc, so members whose role is only implied by a fit still raise E010.
