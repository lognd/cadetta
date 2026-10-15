+++
id = "01M4YNMDMKRAP06VPQP71ZWJXV"
title = "IR decoder so lint FILE runs all IR-only rules (D001, D002, F004, M, N, Q006, S)"
type = "task"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T02:18:04Z"
updated = "2026-10-15T02:26:45Z"
labels = ["creates:crates/cadetta-ir/src/decode.rs", "creates:crates/cadetta-cli/tests/ir_lint_rules.rs", "creates:changelog.d/01M4YNMDMKRAP06VPQP71ZWJXV.added.md"]
scope = ["crates/cadetta-ir/src/lib.rs", "crates/cadetta-cli/src/cmd/ir_lint.rs", "crates/cadetta-cli/src/pipeline.rs", "docs/spec/config-cli.md", "crates/cadetta-ir/src/decode.rs", "crates/cadetta-cli/tests/ir_lint_rules.rs", "changelog.d/01M4YNMDMKRAP06VPQP71ZWJXV.added.md"]

[[acceptance]]
text = "IR JSON decodes into cadetta_ir::model::Project and round-trips"
bound = true

[[acceptance]]
text = "lint FILE runs D001, D002, D003, F004, M, N, Q006 and S over the decoded model"
bound = true
+++

lint FILE (T 5ZFM9K6) only runs D003 via a skeleton decode; the value-reading rules need a full cadetta.ir/1 JSON decoder into cadetta_ir::model.

## Done report

Changed: crates/cadetta-ir/src/decode.rs::decode_project; crates/cadetta-cli/src/cmd/ir_lint.rs::lint_document; crates/cadetta-cli/src/pipeline.rs::s_facts (pub(crate))
Evidence: decode_roundtrips_a_serialized_project, decode_reports_a_json_pointer, clean_document_is_quiet_after_full_decode, inexact_nominal_fires_q006, uncited_ots_value_fires_s001, equal_literals_fire_d001, long_literal_fires_m001
Filed: ~06GWDX5 (undecoded entities, line/col)
Gates: frob check --ticket clean for owned files; remaining findings are other tickets' files
