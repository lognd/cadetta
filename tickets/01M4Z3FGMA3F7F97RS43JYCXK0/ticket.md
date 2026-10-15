+++
id = "01M4Z3FGMA3F7F97RS43JYCXK0"
title = "language: i001/i002/i003 fixtures use cite without accessed (S008); update cites and drop i001-missing-member, i001-optional-member, i001-wrong-kind gaps"
type = "task"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T06:20:04Z"
updated = "2026-10-15T06:27:15Z"
scope = ["tests/conformance/language/i00*", "tests/conformance/known-gaps.toml"]
+++

Found in ~8NXQSDY. I001 already fires for these; only extra S008 on 'cite src: web { url }' (needs accessed per language.md 3.9). Add title+accessed to the i001/i002/i003 fixture cites; then the three I001 fixtures pass. Blocked at time of finding by lease on tests/conformance/** (~9RY31MY).
