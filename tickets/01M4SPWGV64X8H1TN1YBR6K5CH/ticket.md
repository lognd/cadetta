+++
id = "01M4SPWGV64X8H1TN1YBR6K5CH"
title = "IR, JSON schema and digests (cadetta-ir)"
type = "epic"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T04:03:46Z"
updated = "2026-10-13T04:03:46Z"
labels = ["creates:crates/cadetta-ir/**"]
scope = ["crates/cadetta-ir/**"]
+++

Outcome: the tool-agnostic cadetta.ir/1 data model, its committed JSON Schema and BLAKE3 digests.

Success metric: design IR round-trips through JSON byte-identically and the schema validates all fixtures.
