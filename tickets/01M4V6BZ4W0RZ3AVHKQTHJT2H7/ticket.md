+++
id = "01M4V6BZ4W0RZ3AVHKQTHJT2H7"
title = "module tree: E015 reported as E001, N001 false positive on nested modules"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T17:53:35Z"
updated = "2026-10-15T03:09:30Z"
scope = ["crates/cadetta-sema/**", "crates/cadetta-lint/src/rules/n.rs", "crates/cadetta-cli/src/pipeline.rs", "crates/cadetta-cli/tests/module_tree.rs", "tests/conformance/known-gaps.toml", "changelog.d/01M4V6BZ4W0RZ3AVHKQTHJT2H7.fixed.md"]

[[acceptance]]
text = "A bad module file or directory name is reported as code E015, not E001"
bound = true

[[acceptance]]
text = "Valid snake_case nested modules do not raise N001"
bound = false
+++

Found while working ~JKTB9TF. A bad module file or directory name is reported as code E001 with 'E015' inside the message instead of code E015 (packages.md 2, 7). Separately N001 'module name drive::motor breaks the case canon' fires on valid snake_case nested modules (src/drive/motor.cdt, src/bad_dir/inner.cdt). Fixtures: packages/e015-*, module-implicit-parent, module-path-is-file-tree.
