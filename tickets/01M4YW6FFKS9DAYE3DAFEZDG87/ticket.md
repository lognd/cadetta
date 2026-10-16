+++
id = "01M4YW6FFKS9DAYE3DAFEZDG87"
title = "sync: report open O004 obligation for json/csv/sw-equations bindings without drive"
type = "task"
category = "in-progress"
priority = "medium"
points = 3
reporter = "lognd"
created = "2026-10-15T04:12:47Z"
updated = "2026-10-16T17:05:20Z"
scope = ["crates/cadetta-cli/src/cmd/sync.rs", "crates/cadetta-cli/src/universe.rs", "crates/cadetta-obligations/**", "tests/conformance/known-gaps.toml", "docs/spec/obligations.md"]

[[links]]
kind = "blocked-by"
target = "01M4ZAN1F6F1R2DFW1PHMB7P7J"

[[acceptance]]
text = "cadetta sync with a built-in adapter lacking drive (sw-equations, json, csv) records the handed-over file and reports an open O004 info diagnostic (exit 0); --dry-run records nothing"
bound = true

[[acceptance]]
text = "An attest lock record on sync:FILE discharges the open O004"
bound = true
+++

found while working ~EPB7ZY9. Fixtures emitters/{csv,json,sw-equations}/obligation-open-without-drive (json.md 4, csv.md 4, adapters.md 2): cadetta sync prints only a manual note, no open O004 obligation. Overlaps ~JET4TC0 and ~BC7S5KT (binding sync entries have no executor).
