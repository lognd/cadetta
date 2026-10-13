+++
id = "01M4SPX586CJT7FJ3CBH7AF881"
title = "OQ-8: drive with a session holding unsaved changes"
type = "docs"
category = "triage"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T04:04:07Z"
updated = "2026-10-13T04:04:07Z"
labels = ["open-question"]
+++

Open question OQ-8 from the spec. Closes with an ADR in docs/decisions/.

Question: How `drive` handles a CAD session the user has open with unsaved changes.

Recommendation: Refuse and report, never save on the user's behalf.
