+++
id = "01M4V6BZ91RQTVM8FDJKRVD0V6"
title = "parser rejects 'package::' paths in expressions (reserved keyword)"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T17:53:36Z"
updated = "2026-10-15T03:11:56Z"
scope = ["crates/cadetta-syntax/**"]

[[acceptance]]
text = "package:: and super:: paths parse in expressions"
bound = true
+++

Found while working ~JKTB9TF. 'w = package::a::Thing.size' fails with E001 'package is a reserved keyword and cannot be used as a name', but language.md 4 and packages.md 4 make a fully qualified path legal anywhere without a use. Fixtures: packages/e016-fully-qualified-path-checks-visibility, n004-reference-to-was-path.
