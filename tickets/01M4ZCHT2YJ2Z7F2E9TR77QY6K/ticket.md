+++
id = "01M4ZCHT2YJ2Z7F2E9TR77QY6K"
title = "cadetta import: decode frames and instances from adapter observed JSON"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T08:58:36Z"
updated = "2026-10-15T08:58:36Z"
scope = ["crates/cadetta-cli/src/cmd/import.rs"]
+++

found while working ~MNV70G6: decode_observed does not push frames/instances into ObservedBuilder, so end-to-end import never proposes them
