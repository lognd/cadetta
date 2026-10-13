# Hand-produced output (abbreviated): compare and impact --swap, pilot 07 r3

Design: `design.cdt`. Sections follow obligations.md 5.1 order. `ASSUME`
marks a choice the spec still leaves open. Floats from sqrt are 4 s.f.

Fixture (the spec has none): `analysis/airflow_req.out.json` current,
required_flow 24.1m^3/h, system_k 216000Pa*s^2/m^6 (solver inputs do not
read the fan, so S006 never fires). Locks: `source` acked on all
datasheets; `tolerance_review` (covers fit:*, check:*: 12 checks + 1 fit
= 13 nodes) and `thermal_signoff` (covers require:*: airflow_ok) acked
2026-10-08.

## compare AxialFan80 (excerpt)

| | SunonMf80251 (primary) | NoctuaNfA8Pwm | SunonMf80201 |
|---|---|---|---|
| lifecycle | eol 2027-09 | active | active |
| unit_cost / lead_time / moq | 6.85USD / 6wk / 50 | 16.90USD / 2wk / 1 | 4.95USD / 10wk / 100 |
| operating flow (need 24.1) | 32.92 m^3/h | 30.43 | 22.90 |
| airflow_ok (require) | holds | holds | VIOLATES |
| lead_time_ok (worst_lead) | holds 6wk | holds 2wk | VIOLATES 10wk > 8wk |
| screw_pass (fit) | holds [0.1,0.3] | holds | UNDECIDED [0.3,0.5] C002 |
| grommet_pass | holds | UNDECIDED [11.0,11.8] vs [11.3,11.5] C002 | holds |
| viable | primary | yes (I003) | NO (I002: airflow_ok, lead_time_ok) |

All other checks hold for all three. L001 is not raised: the eol primary
has a viable alternate.

## impact --swap acme.cooling::CoolingModule/fan acme.cooling::SunonMf80201

1. verdict: blocked (airflow_ok is a violated require; lead_time_ok is
   waivable but moot)
2. values that change: direct: depth 25->20mm, mount_hole 4.3->4.5mm,
   max_airflow 60->38m^3/h, max_static 40->22Pa, mass 85->62g, unit_cost,
   lead_time, moq, mpn. Derived: screw_stack 28.6->23.6mm, screw_length
   32->25mm, screws[0..3].mass/unit_cost, worst_lead 6->10wk,
   module_cost, module_mass, stack_height, tip_len.
3. verdicts that flip: airflow_ok holds->violates; lead_time_ok
   holds->violates; screw_pass holds->undecided.
4. suggested edit: `part fan: AxialFan80 = SunonMf80201 alt
   [NoctuaNfA8Pwm, SunonMf80251]`
5. CAD re-sync: cooling_module.FCStd (fan.depth, screws[*].length);
   not stale: fan_bracket.FCStd, fan_guard.FCStd
6. attestations invalidated: tolerance_review (12 of 13), thermal_signoff
   (1 of 1; unsatisfiable). source: none. (ASSUME instance-closure rule,
   see FRICTION 1.)
7. commercial: cost 9.046->7.122USD (-1.924, -21.3%); worst_lead
   6->10wk; moq 50->100; mass 180.6->155.6g

## impact --swap ... NoctuaNfA8Pwm

1. verdict: ok with warnings (grommet_pass undecided)
2. changes: inlet_dia 76->74mm, cutout_dia 77->75mm, guard_ring_id
   77->75mm, max_airflow 60->90.9, max_static 40->23.2, lead_len,
   connector_env, mass, cost, lead_time 6->2wk, moq, mpn, supplier;
   operating flow 32.92->30.43; screw_length unchanged (32mm)
3. flips: grommet_pass holds->undecided
4. edit: `... = NoctuaNfA8Pwm alt [SunonMf80201, SunonMf80251]`
5. re-sync: fan_bracket.FCStd (cutout_dia), fan_guard.FCStd
   (guard_ring_id); not stale: cooling_module.FCStd
6. attestations: tolerance_review 12 of 13, thermal_signoff 1 of 1
   (re-ack discharges)
7. commercial: 9.046->19.096USD (+10.050, +111.1%); worst_lead 6->2wk;
   moq 50->1; mass 180.6->173.6g

## Deterministic from the spec?

Mostly. Items 1-4 and 7 are now derivable from language.md 3.2,
solvers.md 4.1 and 5.1. Items 5 and 6 are derivable only after an
assumption: the re-sync list needs bindings files (adapters.md 3, now
fine); the attestation counts need the instance-closure rule for an
interface-typed slot (not stated). Also unfixed: percentage rounding and
float formatting in deltas, and the lack of any fixture format. Verdict:
deterministic for values, verdicts and edits; not yet for attestation
counts and number formatting.
