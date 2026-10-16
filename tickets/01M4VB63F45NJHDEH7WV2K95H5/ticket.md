+++
id = "01M4VB63F45NJHDEH7WV2K95H5"
title = "RFS true position: [position d] on Point2 with disc source variables"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4VB1MJC8NTMD6AAZR5Y5J08"
reporter = "lognd"
created = "2026-10-13T19:17:46Z"
updated = "2026-10-16T07:48:42Z"
persona = "design engineer"
capability = "tolerance hole patterns by true position as drawings do"
outcome_text = "aligned checks match the drawing and are exact for round zones"
idempotency_key = "d15:true-position"
scope = ["crates/cadetta-syntax/src/expr.rs", "crates/cadetta-syntax/src/ast.rs", "crates/cadetta-units/src/**", "crates/cadetta-sema/src/affine.rs", "crates/cadetta-sema/src/point.rs", "crates/cadetta-geometry/src/**", "crates/cadetta-ir/src/model.rs", "docs/schemas/cadetta.ir.1.json", "tests/conformance/geometry/**", "tests/conformance/quantities/**"]

[[links]]
kind = "blocked-by"
target = "01M4SWG6T58Q489WASV5DYGF7F"

[[links]]
kind = "blocked-by"
target = "01M4VB636BKBDGRW3NBMGK7CBZ"

[[links]]
kind = "blocked-by"
target = "01M4VS6WG57MAD0T8GK956NGG9"

[[links]]
kind = "blocked-by"
target = "01M50C0NCKDC43EDJP1BS8CTVB"

[[links]]
kind = "blocked-by"
target = "01M50C0NEZM1R9KYFTRJCJY1Z8"

[[acceptance]]
text = "Given [position d] after a Point2 value or array, when evaluated, then each point gets one disc variable entering x and y with coefficient d/2, and the IR carries paired .u/.v terms and Point.position (geometry.md 3.1; quantities.md 5.4; ir.md 3)"
bound = false

[[acceptance]]
text = "Given position on a Point3, combined with a coordinate tolerance, or with a toleranced generator member, when checked, then E006 (geometry.md 3.1)"
bound = false

[[acceptance]]
text = "Given .x of a position-toleranced point, when bounded, then it is exactly [-d/2, d/2]; a mixed projection after an off-90deg rotation uses the Euclidean norm, marked conservative (quantities.md 5.4)"
bound = false

[[acceptance]]
text = "Given aligned over two coplanar position-toleranced patterns, when evaluated, then the worst offset is the nominal offset plus the sum of radii, exactly (0.1mm for the geometry.md 3.1 example) (geometry.md 3.1)"
bound = false

[[acceptance]]
text = "Given [position], when parsed, then holes[position] remains an index (language.md 5a)"
bound = false
+++
