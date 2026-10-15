+++
id = "01M4W47VTBXFMC70Q2R137W4EE"
title = "Part slot canonical path uses '.' instead of '/' (Rover.wheel vs Rover/wheel)"
type = "bug"
category = "in-progress"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-14T02:35:38Z"
updated = "2026-10-15T07:23:32Z"
scope = ["crates/cadetta-sema/src/lower.rs", "crates/cadetta-cli/src/cmd/impact_swap.rs", "crates/cadetta-ir/src/path.rs", "crates/cadetta-cli/tests/**"]

[[acceptance]]
text = "a part slot lowers to pkg::Assembly/part and impact --swap accepts that form"
bound = false
+++

found while working ~HSDKA6V. lower_part pushes MemberSeg::Name so a slot lowers to acme.shop::Rover.wheel, but the spec, goldens (07-part-swap: CoolingModule/fan) and binding paths use acme.shop::Rover/wheel; impact --swap with the slash form reports 'not a part slot'. Also impact_swap names() is asymmetric for bare alt entries versus full-path NEW (reports 'not yet approved' for an alt-listed part given as full path). cadetta-sema is held by ~4BJ1ZGM.
