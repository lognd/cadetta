+++
id = "01M4ZACVTPPWND6W694CNW9MJ0"
title = "Emit a [*]-bound member under the star path's mangled name"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 1
reporter = "lognd"
created = "2026-10-15T08:20:57Z"
updated = "2026-10-15T08:27:16Z"
scope = ["crates/cadetta-cli/src/cmd/emit.rs", "tests/conformance/known-gaps.toml"]

[[links]]
kind = "blocked-by"
target = "01M4Z92C5D3GN1R52D5JSTNC17"

[[acceptance]]
text = "A member emitted through a [*] binding creates its own global under the mangled star path (Rover__axle__end_diameter), not instance 0"
bound = true
+++

Found while working ~9WDH9XE. emit.rs builds the own-global name from the member path (mangle of Rover/axle[0].x, giving Rover__axle_0__x); adapters.md 3.1 says the [*] segment is dropped, so use the matched binding path (see wildcard/locators_for). Fixture adapters/star-mangle-drops-segment; remove its known-gaps entry. emit.rs is leased by ~JSTNC17 so this waits for it.
