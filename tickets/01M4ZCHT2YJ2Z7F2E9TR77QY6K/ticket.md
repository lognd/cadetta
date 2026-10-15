+++
id = "01M4ZCHT2YJ2Z7F2E9TR77QY6K"
title = "cadetta import: decode frames and instances from adapter observed JSON"
type = "bug"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-15T08:58:36Z"
updated = "2026-10-15T09:14:55Z"
scope = ["crates/cadetta-cli/src/cmd/import.rs", "crates/cadetta-cli/src/cmd/sync.rs"]

[[acceptance]]
text = "decode_observed pushes adapter-observed frames and instances into ObservedBuilder; unit test decodes JSON with frames+instances"
bound = true
+++

found while working ~MNV70G6: decode_observed does not push frames/instances into ObservedBuilder, so end-to-end import never proposes them
