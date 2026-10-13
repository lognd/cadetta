+++
id = "01M4SWN8MPH5ARCRJG4QB9ANVQ"
title = "cadetta lsp: diagnostics and formatting over stdio"
type = "story"
category = "in-progress"
priority = "high"
points = 5
parent = "01M4SPWHHNRSGB9QZ04CMHWRRB"
reporter = "lognd"
created = "2026-10-13T05:44:40Z"
updated = "2026-10-13T22:00:53Z"
persona = "design engineer"
capability = "see problems in any editor"
idempotency_key = "replan14:cli-lsp"
labels = ["deferred", "creates:crates/cadetta-cli/src/cmd/lsp.rs"]
scope = ["crates/cadetta-cli/src/cmd/lsp.rs", "crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-cli/src/lib.rs", "crates/cadetta-cli/tests/**", "crates/cadetta-cli/Cargo.toml", "Cargo.lock", "crates/cadetta-cli/src/main.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SWK9V5CYNNVTAS3PE4EC23"

[[links]]
kind = "blocked-by"
target = "01M4SWKA7HT50RJJ7ZZWP6WMKH"

[[acceptance]]
text = "Given cadetta lsp on stdio, when a client initializes, then it advertises only textDocumentSync (open, full change, save, close), documentFormattingProvider and positionEncoding (utf-32 when offered, else utf-16), logs only to stderr, and exits 0 after shutdown then exit, 1 on exit alone (config-cli.md 3.8)"
bound = true

[[acceptance]]
text = "Given an open or changed .cdt buffer, when parsed, then its E001 diagnostics are published for that document immediately; on open and save the package's full check runs against disk, every package file is published (empty lists clear stale ones), waived diagnostics are omitted, and a superseded run's result is dropped (config-cli.md 3.8)"
bound = true

[[acceptance]]
text = "Given cadetta.diag/1 records, when mapped, then range, severity 1-4, code, source cadetta, message with note and help lines, and data {path, variant} follow config-cli.md 3.8; lock locations publish on cadetta.lock and native or absent locations are logged only"
bound = true

[[acceptance]]
text = "Given textDocument/formatting, when requested, then one whole-document edit with fmt output is returned, none when canonical, and an error response for a buffer with syntax errors (config-cli.md 3.8)"
bound = false
+++
