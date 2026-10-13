+++
id = "01M4SPWG5A0FJNPKRPDDCJQZN8"
title = "Language: syntax and formatter (cadetta-syntax)"
type = "epic"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T04:03:45Z"
updated = "2026-10-13T04:04:06Z"
labels = ["creates:crates/cadetta-syntax/**"]
scope = ["crates/cadetta-syntax/**"]

[[links]]
kind = "blocks"
target = "01M4SPWGB2729B40CQE81MHM2Z"
+++

Outcome: lexing, parsing and canonical formatting of .cdt files per docs/spec/language.md.

Success metric: the language.md example parses, formats idempotently, and fmt never changes the IR digest.
