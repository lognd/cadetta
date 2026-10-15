+++
id = "01M4YW6FFKS9DAYE3DAFEZDG87"
title = "sync: report open O004 obligation for json/csv/sw-equations bindings without drive"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T04:12:47Z"
updated = "2026-10-15T04:12:47Z"
scope = ["crates/cadetta-cli/src/cmd/sync.rs", "crates/cadetta-cli/src/universe.rs", "crates/cadetta-obligations/**", "tests/conformance/known-gaps.toml"]
+++

found while working ~EPB7ZY9. Fixtures emitters/{csv,json,sw-equations}/obligation-open-without-drive (json.md 4, csv.md 4, adapters.md 2): cadetta sync prints only a manual note, no open O004 obligation. Overlaps ~JET4TC0 and ~BC7S5KT (binding sync entries have no executor).
