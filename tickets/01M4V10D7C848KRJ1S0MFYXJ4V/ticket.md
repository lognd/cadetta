+++
id = "01M4V10D7C848KRJ1S0MFYXJ4V"
title = "Conformance form for digest rules (ir.md 4) and schema classify"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:19:54Z"
updated = "2026-10-15T03:05:35Z"
scope = ["docs/spec/**", "tests/conformance/README.md", "changelog.d/01M4V10D7C848KRJ1S0MFYXJ4V.*"]

[[acceptance]]
text = "tests/conformance/README.md defines a compare form for ir.md 4 digest rules and ir.md 5 classify(old,new)"
bound = true
+++

found while working ~99NMCBG: ir.md 4 digest rules (formatting never changes a digest, semantic vs presentational attrs, closure Merkle, swap staleness) and 5 classify(old,new) need two-input comparisons that the expect.json contract cannot express. Add an entry (digest compare of two fixtures) to tests/conformance/README.md.
