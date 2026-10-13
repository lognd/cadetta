+++
id = "01M4SPWSABPXKK62B5V0SMJ8FD"
title = "Built-in SOLIDWORKS linked-equations adapter"
type = "story"
category = "todo"
priority = "medium"
points = 5
parent = "01M4SPWH74K9QQS72Z9YMTPD72"
reporter = "lognd"
created = "2026-10-13T04:03:55Z"
updated = "2026-10-13T04:04:04Z"
persona = "design engineer"
capability = "emit and read an equations text file for Link to external file"
outcome_text = "SOLIDWORKS globals update without retyping"
labels = ["creates:crates/cadetta-adapter/src/builtin/sw_equations.rs"]
scope = ["crates/cadetta-adapter/src/builtin/sw_equations.rs"]

[[links]]
kind = "blocks"
target = "01M4SPWTA1PQV15S63741V30XT"

[[acceptance]]
text = "Given `Axle.end_diameter`, when emitted, then the file contains the global variable `Axle_end_diameter` with its value in mm"
bound = false

[[acceptance]]
text = "Given an equations file, when read, then observed IR carries native names and locators"
bound = false
+++
