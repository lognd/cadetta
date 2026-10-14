+++
id = "01M4Y42EVP0N8T8JFC13E4TV99"
title = "Wire import --update/--write into the CLI"
type = "task"
category = "done"
outcome = "wont-fix"
priority = "medium"
points = 5
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-14T21:11:10Z"
updated = "2026-10-14T21:19:06Z"
idempotency_key = "cli-import-update-wire"
scope = ["crates/cadetta-cli/src/cmd/import*"]

[[links]]
kind = "blocked-by"
target = "01M4SWMCWHXKA8K6VG7Z1PNF3W"

[[acceptance]]
text = "Given import ADAPTER FILE --update, when CAD values changed, then the literal diff is printed and no file is written"
bound = false

[[acceptance]]
text = "Given --write, when run, then sources are rewritten and the lock synced values updated"
bound = false
+++

import::update (plan, render_diff, apply) is pure and unwired. The CLI must resolve each bound path to a Source from the sema AST (literal span, toleranced, derived, cited), build Entries from lock base and observed values, print the diff for --update, and on --write rewrite sources via apply and record lock_updates; nothing is written otherwise (config-cli.md 3, adapters.md 7.4).
