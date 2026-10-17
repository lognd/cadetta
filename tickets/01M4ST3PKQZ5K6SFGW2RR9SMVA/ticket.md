+++
id = "01M4ST3PKQZ5K6SFGW2RR9SMVA"
title = "OQ-14: Map a ScrewJoint onto native hole features"
type = "docs"
category = "triage"
priority = "medium"
reporter = "lognd"
created = "2026-10-13T05:00:07Z"
updated = "2026-10-16T23:38:20Z"
labels = ["open-question"]
+++

Open question OQ-14 from stdlib.md. Closes with an ADR in docs/decisions/.

Question: Should adapters map a whole joint onto a native hole feature (SOLIDWORKS Hole Wizard, FreeCAD PartDesign Hole, Onshape hole) rather than numeric parameters?

Recommendation: yes, after 0.1.0, as an optional adapter capability.
