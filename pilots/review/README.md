# Final review round (draft 1.3 to 1.4)

Five reviewers each read the whole specification from one angle. The
reports are the files here; the rulings are `draft-1-4-decisions.md`;
the decision record is `docs/decisions/2026-10-13-draft-1-4-decisions.md`.

| Report | Angle | Rating | Blocker / gap / polish |
|---|---|---|---|
| review-1-journey.md | SOLIDWORKS user, import to release | 2/5 | 4 / 7 / 1 |
| review-2-obligations.md | Obligations over time, diagnostics UX | 3/5, 2/5 | 3 / 8 / 1 |
| review-3-geometry.md | Positions and datums, with a pilot | 3/5 | 3 / 7 / 2 |
| review-4-cohesion.md | Cross-document cohesion | 4/5 | 2 / 9 / 1 |
| review-5-scale-python.md | Project scale and Python | 2.5/5 | 4 / 7 / 1 |
| verify-1-4.md | Seams after the parallel edit | - | 2 / 11 / 4 |

Every finding was ruled on. All were accepted except one: sync bases
live in the committed lock rather than a per-machine file, because they
describe committed CAD files.
