+++
id = "01M4Z6SFCH83BW0D8GPR501QAM"
title = "Conformance executor impact_swap passes dotted slot Rover.wheel; must be Rover/wheel once ~137W4EE lands"
type = "bug"
category = "todo"
priority = "medium"
reporter = "lognd"
created = "2026-10-15T07:17:56Z"
updated = "2026-10-15T07:45:24Z"
+++

In crates/cadetta-cli/tests/conformance/binding.rs (~line 297) change format!("{package}::Rover.wheel") to format!("{package}::Rover/wheel"). Required by ~137W4EE (slots lower to Type/part); until then fixture adapters/slots-swap-impact-replace-component fails. Not editable by ~137W4EE because ~Q6AKDVQ holds crates/cadetta-cli/tests/**.
