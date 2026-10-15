+++
id = "01M4Z92C25BYMP5JYSX9WDH9XE"
title = "Part-array instance members are not lowered to IR paths, so emitters and bindings see none"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T07:57:44Z"
updated = "2026-10-15T07:57:44Z"
scope = ["crates/cadetta-sema/src/**"]
+++

found while working ~6ES67RW: with valid syntax 'part axle[2]: Axle' emit csv/json/sw-equations write no Rover/axle[i].member rows and [*] / [1] bindings never match. Blocks fixtures adapters/star-binding-emitted-once-at-instance-0, adapters/star-mangle-drops-segment, adapters/r012-star-binding-differing-instances (must-not-fire), emitters/{csv,json}/name-mangled-path, emitters/sw-equations/naming-component-field.
