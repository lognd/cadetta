+++
id = "01M4V14BY8H42AS84MBAN7BAC5"
title = "check reports E003/E005 as exit-2 load errors instead of diagnostics"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T16:22:03Z"
updated = "2026-10-15T03:07:58Z"
scope = ["crates/cadetta-cli/**"]

[[acceptance]]
text = "cadetta check reports E005 (missing src, member or path dependency without cadetta.toml) as a fixed-error diagnostic with exit 1 and a report, not an exit-2 load error"
bound = true
+++

found while working ~5GAW3SB. Spec config-cli.md 1 (3,4) and lints.md catalog make E003 and E005 ordinary fixed-error diagnostics (exit 1 at fail_on, JSON report with path/location). Implementation aborts check with exit 2 and no report. Fixtures config-cli/e003, e005-no-cdt, e005-member, e005-path-dep are listed in known-gaps.toml.
