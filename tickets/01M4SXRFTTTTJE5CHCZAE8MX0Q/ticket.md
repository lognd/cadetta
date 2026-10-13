+++
id = "01M4SXRFTTTTJE5CHCZAE8MX0Q"
title = "File-tree modules, canonical paths and E015"
type = "story"
category = "in-progress"
priority = "medium"
points = 5
parent = "01M4SXQPADRSNTEY3J7PH899Y8"
reporter = "lognd"
created = "2026-10-13T06:03:54Z"
updated = "2026-10-13T07:22:25Z"
persona = "library author"
capability = "organise a package as files with no declarations"
labels = ["creates:crates/cadetta-sema/src/modules.rs"]
scope = ["crates/cadetta-sema/src/modules.rs", "docs/spec/packages.md", "crates/cadetta-sema/src/lib.rs"]

[[links]]
kind = "blocked-by"
target = "01M4SPWP1JK8XH8EKXK71P7EFH"

[[links]]
kind = "blocked-by"
target = "01M4SPWPCW1C0QC67GZG99EHTZ"

[[links]]
kind = "blocked-by"
target = "01M4SXQWGAYJDGMGKNBMHASNBA"

[[acceptance]]
text = "Given src/lib.cdt, src/frame.cdt and src/drive/motor.cdt in package acme.rover, when loaded, then modules are acme.rover, acme.rover::frame and acme.rover::drive::motor, and drive.cdt may be absent (packages.md 2)"
bound = true

[[acceptance]]
text = "Given a file or directory under src that is not snake_case, when loaded, then E015 fires (packages.md 2, 7; N001)"
bound = true

[[acceptance]]
text = "Given an item in a module, when its path is built, then it is package::module::Item and members follow, as in acme.rover::drive::motor::Gm42A.rated_torque (packages.md 2; ir.md 2)"
bound = true

[[acceptance]]
text = "Given matches under src, when compiled, then files are sorted by path so digests are deterministic; a component defined twice in one module is E003 with both spans (config-cli.md 1.3, 1.4)"
bound = true

[[acceptance]]
text = "Given a path to a bindings, solver script or cited file, when resolved, then it is relative to the package directory, not src (packages.md 2)"
bound = true
+++
