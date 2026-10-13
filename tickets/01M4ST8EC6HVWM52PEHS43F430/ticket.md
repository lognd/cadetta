+++
id = "01M4ST8EC6HVWM52PEHS43F430"
title = "Variant evaluation engine (slot at a time)"
type = "story"
category = "in-progress"
priority = "medium"
points = 8
parent = "01M4ST8E5XX0TX0MBND07JQME1"
reporter = "lognd"
created = "2026-10-13T05:02:43Z"
updated = "2026-10-13T08:32:31Z"
persona = "design engineer"
capability = "re-evaluate the design with each alternate in place of the primary"
idempotency_key = "replan13:Variant evaluation engine (slot at a time)"
labels = ["creates:crates/cadetta-variants/src/evaluate.rs"]
scope = ["crates/cadetta-variants/src/evaluate.rs"]

[[links]]
kind = "blocks"
target = "01M4ST8EEGK712KZT44DEDBSEG"

[[links]]
kind = "blocks"
target = "01M4ST8F91FHYFC01TN6018BAM"

[[links]]
kind = "blocks"
target = "01M4ST8FCNN3GZKZNRK2X3Q55S"

[[links]]
kind = "blocks"
target = "01M4STANBCA3XCNHR3X1NJKGFE"

[[acceptance]]
text = "Given a slot with alternate A, when evaluated, then A replaces the primary in every instance of the slot, all other slots keep their primaries, and derived values, part arguments, fits, checks, requires and roll-ups are re-evaluated (language.md 3.2)"
bound = false

[[acceptance]]
text = "Given N alternates across slots, when evaluated, then the work is linear (one variant evaluation per alternate), never a cross product (language.md 3.2)"
bound = false

[[acceptance]]
text = 'Given values that differ from the primary, when stored, then only the differing values are kept per variant keyed "slot=alternate" (ir.md 3, language.md 3.2)'
bound = false

[[acceptance]]
text = "Given an evaluation error under a variant (table lookup misses, failed `as`), when evaluated, then that alternate is non-viable with the error as reason and the run does not fail (language.md 3.2)"
bound = false

[[acceptance]]
text = "Given a solver whose inputs change under a variant, when evaluated, then the variant reads its own result or is unknown (solvers.md 4.1)"
bound = false
+++
