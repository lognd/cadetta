+++
id = "01M4VB630NZKXA6Q9D7253FATR"
title = "Solver script statement replaces the script citation"
type = "story"
category = "in-progress"
priority = "medium"
points = 3
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T19:17:46Z"
updated = "2026-10-13T21:44:45Z"
persona = "design engineer"
capability = "name a solver's script as an input rather than a citation"
outcome_text = "citations only ever mean sources"
idempotency_key = "d15:solver-script"
scope = ["crates/cadetta-syntax/src/**", "crates/cadetta-sema/src/**", "crates/cadetta-solve/src/**", "tests/golden/07-part-swap/**", "tests/golden/08-gearmotor-module/**", "tests/conformance/solvers/**", "crates/cadetta-cli/src/cmd/solve.rs", "crates/cadetta-syntax/tests/**", "docs/spec/solvers.md"]

[[links]]
kind = "blocked-by"
target = "01M4VB62VJQV9AKSSQME8NWW6Y"

[[acceptance]]
text = 'Given solver NAME [@cite] { script "path" ... }, when compiled, then Solver.script is the path, its digest is an input and Solver.citation is the optional CiteRef (solvers.md 1; ir.md 3)'
bound = true

[[acceptance]]
text = 'Given a solver body without script, or the draft 1.4 form solver NAME @"path", when parsed, then E001 (language.md 5a)'
bound = true

[[acceptance]]
text = "Given golden cases 07 and 08 and the solvers fixtures, when run, then they use the script statement and pass"
bound = true
+++
