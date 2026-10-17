+++
id = "01M4ST3P9EKAA7Y6MZGXWHXNTJ"
title = "OQ-11: Packages for shared parts libraries"
type = "docs"
category = "triage"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T05:00:07Z"
updated = "2026-10-16T23:38:20Z"
labels = ["open-question"]
+++

Open question OQ-11 from language.md. Closes with an ADR in docs/decisions/.

Question: How are shared parts libraries versioned and fetched?

Recommendation: std ships with the compiler (stdlib.md); third-party packages wait past 0.1.0.
