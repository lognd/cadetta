+++
id = "01M4V6C5E3CF67BVGD3SE9XKVS"
title = "packages.md: diagnostic codes for registry source, reserved std name and ignored differing source"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-13T17:53:42Z"
updated = "2026-10-16T07:22:15Z"
scope = ["docs/spec/**"]

[[acceptance]]
text = "packages.md 1 and 7 and lints.md E005 name the codes for registry source, reserved std name and ignored differing source"
bound = true
+++

Spec ambiguity found while writing ~JKTB9TF fixtures. packages.md 1 says a registry-only dependency is 'rejected' and that no package may be named std, and that a later different source for a selected package is 'ignored with a warning', but none names a diagnostic code. Fixtures assume E005 for the first two (packages/e005-registry-dependency-rejected, e005-std-name-reserved); the warning has no fixture. Decide the codes and write them into packages.md 1 and 7.
