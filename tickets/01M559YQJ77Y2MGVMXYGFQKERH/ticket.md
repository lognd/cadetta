+++
id = "01M559YQJ77Y2MGVMXYGFQKERH"
title = "SwComDocumentReader.ReadDimension scales pattern counts by 1000 on read"
type = "bug"
category = "done"
outcome = "done"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-17T16:08:40Z"
updated = "2026-10-17T16:39:43Z"
scope = ["adapters/solidworks/**", "changelog.d/01M559YQJ77Y2MGVMXYGFQKERH.*"]

[[acceptance]]
text = "Given a scalar (pattern count) dimension, when read through SwComDocumentReader, then it reads as an unscaled unitless number"
bound = true
+++

Read path still scales scalar (swScalarDimension) dimensions by the length factor with a mm unit; mirror the push-path fix (DimensionUnits).
