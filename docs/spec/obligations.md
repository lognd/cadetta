# The obligation graph

Core functionality. Every claim cadetta makes about a design (this value
came from that datasheet, this CAD file reflects this design, this check
holds, this reviewer signed off) is an obligation pinned to the digests of
what it depends on. When anything upstream changes, exactly the
obligations that depended on it become stale, and cadetta can say which,
why, and what discharges them.

## 1. Nodes and edges

Nodes are IR entities, each with a digest (ir.md 4): parameters, bounds,
citations, tables, fits, checks, requires, solvers and their result
files, parts and their alternates, bindings, native CAD files, inspection
data sets, and attestations.

Edges:

| Edge | From -> to | Created by |
|---|---|---|
| `derives` | param -> each param its expression reads | the compiler |
| `cites` | param, table, component -> citation | `@` |
| `implements` | component -> interface | `: Iface` |
| `instantiates` | part -> primary and each alternate | `part` |
| `evaluates` | check, require, fit -> what it reads | the compiler |
| `solves` | solver output -> solver inputs, script, deps | `solver` |
| `binds` | param -> native locator | bindings |
| `realizes` | native file -> bound params | adapter `read` |
| `attests` | attestation -> a set of node digests | `cadetta ack` |
| `waives` | waiver -> target | `waive` |

## 2. Obligations and states

| Obligation | Discharged by | Stale when |
|---|---|---|
| check / require / fit holds | evaluation, every run | never stale; recomputed |
| alternate is viable | variant evaluation (language.md 3.2): nothing violates | never stale; recomputed |
| solver result is current | `cadetta solve` | input, script or dep digest changed |
| CAD file reflects design | `cadetta sync` (or `emit` plus read-back) | a bound param's emitted value changed |
| value matches its citation | `cadetta ack PATH --role source` | the cited local file's digest changed, or the citation text changed |
| review / sign-off | `cadetta ack SELECTOR --role NAME` | any node in the attested set changed digest |
| requirement is met | a `require` evaluating `holds`, plus its attestation if configured | the require or anything it reads changed |
| as-built conforms | inspection data read as observed IR, within tolerance | the design changed after the inspection |

States: `discharged`, `stale` (upstream changed since discharge),
`violated`, `undecided`, `open` (never discharged), `waived`.

Staleness is computed, never stored: an obligation records the closure
digests (ir.md 4) of the nodes it covers when it was discharged;
comparing them with the current graph gives the state. Because closure
digests are Merkle over `deps`, an attestation on a check goes stale
when anything the check reads changes, however far upstream. This makes the graph rebuildable from source plus the lock
file at any commit.

## 3. The lock file

`cadetta.lock` is committed and holds everything that is not derivable
from source: attestations (who, role, when, reason, the digest set), the
digests of cited local files at acknowledgement, solver result digests,
and last-sync digests per adapter and file. Entries are sorted and
line-oriented so merges are reviewable. Nothing else writes state.

## 4. Roles and policy

`[obligations]` in `cadetta.toml` declares roles and what they must
attest:

```toml
[obligations.roles.tolerance_review]
covers = ["fit:*", "check:*"]
required_for = "release"

[obligations.roles.source]
covers = ["ots:*"]
required_for = "check"
```

`cadetta check` fails on stale or open obligations whose role is
`required_for = "check"`; `cadetta check --release` also fails on the
`release` ones. Attestations are the only way to waive a `require`.

## 5. Queries

| Command | Answers |
|---|---|
| `cadetta why PATH` | Upstream: the derivation chain from a value back to citations, tables, solvers and requirements. |
| `cadetta impact PATH` | Downstream: everything that goes stale or changes verdict if PATH changes. |
| `cadetta impact --swap PART NEW` | A swap preview: values that change, checks and fits that flip, alternates that become primary, CAD files to re-sync, attestations invalidated, and the commercial deltas (cost, lead time, mass). |
| `cadetta compare IFACE` | Every implementer of an interface side by side: interface members, bounds, commercial parameters, lifecycle, and which checks hold for each in this design. |
| `cadetta obligations [--stale] [--role R]` | The open and stale obligation list: the design's to-do list. |
| `cadetta ack SELECTOR --role R --reason "..."` | Discharge attestation obligations at current digests. |

All queries emit JSON (`--format json`) so other tools, including frob,
can consume them. Their schemas live in `docs/schemas/` (`impact.json`,
`compare.json`) and are generated from the Rust types.

### 5.1 `impact --swap` semantics

The preview is the variant evaluation of `PART=NEW` (language.md 3.2),
diffed against the primary evaluation. `NEW` must already implement the
part's interface; it need not be in the `alt` list (then the output says
"not yet approved"). Output sections, in this order, each sorted by
canonical path:

1. **verdict**: `ok`, `ok with warnings` (undecided results), or
   `blocked` (a violation; a violated `require` cannot be waived).
2. **values that change**: direct (interface members) then derived,
   with old and new values; a `let` is listed like any value.
3. **verdicts that flip**: checks, fits, requires, bounds.
4. **suggested edit**: the new `part` line, `NEW` removed from `alt`
   and the old primary appended.
5. **CAD files to re-sync**: files bound to any path whose emitted value
   changes, with the paths; files with no changed bound value are
   listed as not stale.
6. **attestations invalidated**: each attestation whose covered closure
   digests would change, with the count of covered nodes affected.
7. **commercial deltas**: per currency, the sum of `unit_cost * count`
   over the assembly; lead time as the maximum over parts that declare
   it; `moq` per changed part; mass via `mass?(self)` with parts lacking
   mass listed. Rolled up at the assembly named by `PART`'s prefix.

`compare IFACE` is the same evaluation for every implementer of the
interface in every slot typed by it, as a table.

## 6. frob integration (optional)

Cadetta MAY act as a frob sibling (like grimble): `frob check` would run
`cadetta check` and gate on its stale obligations, and frob tickets could
bind to canonical paths (`acme.rover::Axle.end_diameter`). Cadetta never
requires frob.

## Open questions

- OQ-12: Should part swaps be recorded as first-class events (who, why,
  which alternate, cost delta) in the lock file for audit, beyond the
  git history of the `part` line?
- OQ-13: Inspection data formats (CMM CSV, QIF) and how measured
  features bind to canonical paths.
