+++
id = "01M4ST3P32HPTS29XTWX290VXC"
title = "OQ-9: Matched-lot source variables for part instances"
type = "docs"
category = "triage"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T05:00:07Z"
updated = "2026-10-16T23:38:20Z"
labels = ["open-question"]
+++

Open question OQ-9 from quantities.md. Closes with an ADR in docs/decisions/.

Question: Per-instance source variables (quantities.md 5.4) treat two instances of one part as varying independently. Is a matched-set (same lot) mode needed?

Recommendation: not in 0.1.0; pilot r2-02 used the independent (conservative) analysis.
