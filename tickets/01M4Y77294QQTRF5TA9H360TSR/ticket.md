+++
id = "01M4Y77294QQTRF5TA9H360TSR"
title = "Pipeline evaluates each part slot's approved alternates and fills Member.variants / Verdicts.per_alternate"
type = "task"
category = "done"
outcome = "done"
priority = "medium"
points = 5
reporter = "lognd"
created = "2026-10-14T22:06:07Z"
updated = "2026-10-15T10:02:37Z"
scope = ["crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-sema/src/lower.rs", "crates/cadetta-sema/src/eval.rs", "docs/spec/**", "crates/cadetta-cli/tests/alternate_verdicts.rs"]

[[links]]
kind = "blocked-by"
target = "01M4Y77276XRHD2NDXHJMCB8NQ"

[[acceptance]]
text = "pipeline analyze evaluates every part slot alternate (key <slot>=<alt>) and passes LowerInput.variants so Member.variants and Verdicts.per_alternate are populated for checks that differ from the primary"
bound = true

[[acceptance]]
text = "cadetta check reports I002/I003 style diagnostics per non-viable alternate through the pipeline"
bound = true
+++

Split from ~Q1ZBN9Y (title: Fits and variant evaluations not derived for lowering). pipeline.rs evaluate_stages still passes LowerInput.variants = BTreeMap::new(). Plan: per in-scope component/Part with a non-empty alts, evaluate with the slot's primary replaced by each alternate Ctor (fresh Resolver+Evaluator over cloned ast::File), keyed '<slot>=<alternate ctor text>' as lower.rs ctor_text and impact_swap matches_key expect. Blocked by the through-part-read ticket (slot primary never consulted by the evaluator, E002 on 'wheel.load_rating'). Also decide the interaction with processes compensation and parameterized instances, and whether to reuse the cadetta-variants crate (not currently a dependency of cadetta-cli/cadetta-sema).
