# Design history

How the specification got to its current shape. The decisions
themselves are in `docs/decisions/`; this directory keeps the evidence
behind them.

| File | What it is |
|---|---|
| `pilot-round-1.md` | Six pilot designs written against draft 0, and what they found |
| `pilot-round-2.md` | Pilots 02, 04 and 05 against draft 1, the first part-swap pilot, and pilot 08 |
| `pilot-round-3.md` | All eight pilots against draft 1.2 |
| `review-round/` | Five whole-spec reviews against draft 1.3, the rulings that made draft 1.4, and the verification pass |

The pilots' design files and friction reports were removed when the
pilots became the golden corpus (`tests/golden/`). They are in git
history: `git show 9088402:pilots/` lists them, and
`git checkout 9088402 -- pilots` restores them.

Paths, codes and syntax quoted in these files are as they were at the
time. The current spec is in `docs/spec/`.
