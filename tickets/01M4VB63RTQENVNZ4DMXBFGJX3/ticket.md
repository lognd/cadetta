+++
id = "01M4VB63RTQENVNZ4DMXBFGJX3"
title = "cite list (IEEE, BibLaTeX, JSON) and cite pin for typed citations"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T19:17:46Z"
updated = "2026-10-13T22:56:06Z"
persona = "design engineer"
capability = "print the design's reference list and vendor a web source"
outcome_text = "a design review has a bibliography and a URL can be drift-checked"
idempotency_key = "d15:cite-list"
labels = ["creates:docs/schemas/cite-list.json"]
scope = ["crates/cadetta-cli/src/cmd/cite.rs", "crates/cadetta-cli/src/cmd/mod.rs", "crates/cadetta-cli/src/lib.rs", "docs/schemas/cite-list.json", "tests/conformance/config-cli/**", "docs/schemas/cite-list.json", "docs/spec/config-cli.md"]

[[links]]
kind = "blocked-by"
target = "01M4VB62VJQV9AKSSQME8NWW6Y"

[[acceptance]]
text = "Given a design, when cadetta cite list runs, then reached citations are numbered by the language.md 3.9.1 walk and rendered in IEEE plain text per type, with separators collapsed and partial dates at their precision (language.md 3.9.1)"
bound = true

[[acceptance]]
text = "Given --format bibtex or json, when run, then BibLaTeX entries keyed by canonical path, or cite-list.json validated against its schema, are written (language.md 3.9.1; config-cli.md 3)"
bound = false

[[acceptance]]
text = "Given cite pin NAME on a cite with a url, when run, then the bytes land in cites/<16 hex>-<name>, file is set, url kept and accessed added when absent; no url, unknown or ambiguous NAME exits 2 (config-cli.md 3)"
bound = false
+++
