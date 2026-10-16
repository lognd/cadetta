+++
id = "01M4V6C5J6TZ5CVW5YMXXKBNB8"
title = "packages.md: no conformance entry for cadetta update and section 6 trust or attested-upstream"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-13T17:53:42Z"
updated = "2026-10-16T07:25:49Z"
scope = ["docs/spec/**"]
+++

Spec gap found while writing ~JKTB9TF fixtures. packages.md 1 (cadetta update rules: drop records, NAME not in closure exit 2, source forms) and 6 (trust roles, attested upstream in cadetta obligations) are normative but the conformance README only runs check; the expect.json format has no entry for update or for obligations output. Decide the entries and the expectation fields, then add fixtures.
