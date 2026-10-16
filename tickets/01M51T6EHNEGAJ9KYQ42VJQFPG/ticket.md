+++
id = "01M51T6EHNEGAJ9KYQ42VJQFPG"
title = "Waive with a valid future expiry does not suppress C001 in check entry (language/e001-waive-date)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-16T07:35:33Z"
updated = "2026-10-16T12:10:43Z"
scope = ["crates/cadetta-sema/**", "tests/conformance/known-gaps.toml", "crates/cadetta-cli/tests/conformance/run.rs"]

[[acceptance]]
text = "language/e001-waive-date must-not-fire passes; gap entry removed"
bound = false

[[acceptance]]
text = "conformance runner ignores waived diagnostics; language/e001-waive-date passes and is removed from known-gaps.toml"
bound = false
+++

conformance run: language/e001-waive-date [language]: must-not-fire: extra C001 (error) at acme.demo::Part.ok despite 'waive C001 on acme.demo::Part.ok ... expires 2099-01-01'. Found while fixing ~S1S3XZK.
