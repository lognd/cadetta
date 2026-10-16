+++
id = "01M4YTTRS6JMNTHS4QXB08JM3F"
title = "conformance: emitters/json|csv note-emit-point need hole/shaft role from fit class (H7) for emit(mmc)"
type = "bug"
category = "todo"
priority = "medium"
points = 2
reporter = "lognd"
created = "2026-10-15T03:48:55Z"
updated = "2026-10-16T13:06:29Z"
+++

Found while working ~TQMFETK. fixtures emitters/{json,csv}/note-emit-point must_fire use '12mm H7 #[emit(mmc)]' with no #[role]; emit fails E010 (mmc/lmc needs a role). The role should be inferred from the fit class (H = hole, h = shaft). Also emitters/json/note-emit-point.must-not-fire and note-fit-class.must-not-fire exclude any note for '12mm H7' at nominal, contradicting emitters/csv/note-fit-class (note '12mm H7') and the spec; fix the json fixtures.
