# Hand-produced output: compare and impact --swap (pilot 07)

Design: `design.cdt` (namespace `acme.cooling`). Everything below is what
obligations.md 5 says the tools answer, produced by hand. Where the spec
did not fix a choice, the assumption is stated as `ASSUME`. Float digits
come from sqrt and are shown to 4 significant digits (inexact, Q006).

## 0. Fixed context (assumed state, since the spec has no state fixture)

Solver result `analysis/airflow_req.out.json` (committed, current):
`required_flow = 24.1m^3/h` (60W / (1.164 kg/m3 * 1005 J/kgK * 10K) * 1.3,
rounded up to 0.1), `system_k = 216000Pa*s^2/m^6` (15Pa at 30m^3/h).

`cadetta.toml` roles: `source` covers `ots:*` (check), `tolerance_review`
covers `fit:*`, `check:*` (release), `thermal_signoff` covers `require:*`
(release). `cadetta.lock`: `source` acked on all five datasheets and
tables 2026-10-05 (alternates are datasheet-pinned too); `tolerance_review`
and `thermal_signoff` acked 2026-10-08 at the current digests. Last-sync
digests recorded for `fan_bracket.FCStd`, `fan_guard.FCStd`,
`cooling_module.FCStd` (read-back) and `build/bom.csv` (emit).

Operating points (square-law system, linear fan curve): Sunon25 32.92,
Noctua 30.43, Sunon20 22.90 m^3/h, against required 24.1.

## 1. `cadetta compare AxialFan80`

```
acme.cooling::AxialFan80   implementers: 3   primary of CoolingModule/fan: SunonMf80251

member            SunonMf80251 (primary)  NoctuaNfA8Pwm        SunonMf80201
----------------  ----------------------  -------------------  ---------------------
mpn               MF80251V1-1000U-A99     NF-A8 PWM            MF80201V1-1000U-A99
width  [79.5..80.5]   80 +-0.5mm   ok      80 +-0.5mm   ok      80 +-0.5mm   ok
hole_pitch [71..72]   71.5 +-0.2mm ok      71.5 +-0.2mm ok      71.5 +-0.2mm ok
depth             25 +-0.5mm              25 +-0.5mm           20 +-0.5mm
mount_hole        4.3 [+0.1 -0]mm         4.3 [+0.1 -0]mm      4.5 [+0.1 -0]mm
inlet_dia         76mm                    74mm                 76mm
rated_voltage     12V                     12V                  12V
max_airflow       60m^3/h (35.3 cfm)      90.9m^3/h            38m^3/h
max_static        40Pa                    23.2Pa               22Pa
lead_len          300 +-10mm              200 +-10mm           300 +-10mm
connector_env     8 +-0.3mm               11.4 +-0.4mm         8 +-0.3mm
mass              85g                     78g                  62g

commercial
unit_cost         6.85USD                 16.90USD             4.95USD
lead_time         6wk                     2wk                  10wk
moq               50                      1                    100
supplier          Sunon                   Noctua               Sunon

lifecycle         eol 2027-09             active (2026-10)     active (2026-10)

checks (design context: CoolingModule)
airflow_ok (require)  holds  32.92         holds  30.43         VIOLATES  22.90 < 24.1
screw_pass (fit)      holds  [0.1,0.3]     holds  [0.1,0.3]     UNDECIDED [0.3,0.5] vs [0.05,0.45]  C002
grommet_pass          holds  [7.7,8.3]     UNDECIDED [11.0,11.8] vs [11.3,11.5]  C002   holds
lead_time_ok          holds  6wk           holds  2wk          VIOLATES  10wk > 8wk
keepout_ok            holds  <=29.7        holds  <=29.7       holds  <=24.7
tip_ok                holds  <=6.1         holds  <=6.1        holds  <=4.1
lead_reach_ok         holds                holds                holds
moq_ok, supply_ok, engage_ok, sheet_ok, pilot_ok, guard_ok   holds (all three)

viable alternate?   (primary)             yes (no violation, 1 C002)   NO (I002: airflow_ok, lead_time_ok)
diagnostics         L001 not raised: primary is eol but NoctuaNfA8Pwm is viable
                    I002 SunonMf80201 not viable; C002 for alt NoctuaNfA8Pwm: grommet_pass;
                    C002 for alt SunonMf80201: screw_pass
```

ASSUME: "viable" = no check violated for that alternate (undecided
counts as viable with a warning). The spec says only "evaluation of
every check for that alternate".

## 2. `cadetta impact --swap acme.cooling::CoolingModule/fan acme.cooling::SunonMf80201` (cost pressure)

```
swap CoolingModule/fan: SunonMf80251 -> SunonMf80201
verdict: BLOCKED (not promotable: 2 violated, 1 undecided; require airflow_ok cannot be waived)

values that change (direct, from interface)
  fan.mpn            MF80251V1-1000U-A99 -> MF80201V1-1000U-A99
  fan.depth          25 +-0.5mm -> 20 +-0.5mm
  fan.mount_hole     4.3 [+0.1 -0]mm -> 4.5 [+0.1 -0]mm
  fan.max_airflow    60 -> 38 m^3/h        fan.max_static  40 -> 22 Pa
  fan.mass           85g -> 62g
  fan.unit_cost 6.85 -> 4.95USD   fan.lead_time 6wk -> 10wk   fan.moq 50 -> 100
  (unchanged: width, hole_pitch, inlet_dia, voltage, lead_len, connector_env, supplier)
values that change (derived)
  FanBracket.screw_stack     28.6mm -> 23.6mm
  FanBracket.screw_length    32mm -> 25mm
  CoolingModule/screws[0..3].length  32 -> 25mm ; .mass 2.4g -> 1.9g ; .unit_cost 0.044 -> 0.038USD
  operating_flow(fan)        32.92 -> 22.90 m^3/h   (inexact)
  unchanged derived: cutout_dia 77mm, plate_w 100mm, pitch 71.5mm, guard_ring_id 77mm, pilot_dia

checks and fits that flip
  airflow_ok    holds -> VIOLATES  (22.90 < 24.1 m^3/h; solver bound)   [require]
  lead_time_ok  holds -> VIOLATES  (10wk > 8wk)
  screw_pass    holds -> UNDECIDED (clearance [0.3,0.5]mm vs [0.05,0.45]) C002
  unchanged: keepout_ok tip_ok engage_ok lead_reach_ok grommet_pass moq_ok supply_ok sheet_ok pilot_ok guard_ok

alternates that become primary: SunonMf80201 (not viable, needs waiver; require not waivable)
  resulting alt list: [NoctuaNfA8Pwm]   (ASSUME: old primary is dropped; see FRICTION)
CAD files to re-sync
  cad/freecad/cooling_module.FCStd   fan.depth 25->20mm, screws[*].length 32->25mm
  build/bom.csv                       (emit) mpn, unit_cost, screw length
  not stale: fan_bracket.FCStd, fan_guard.FCStd (no bound emitted value changes)
attestations invalidated
  tolerance_review@2026-10-08  stale (9 of its 12 nodes read the fan, transitively)
  thermal_signoff@2026-10-08   stale and unsatisfiable: require airflow_ok violated
  source: none (ots digests unchanged; SunonMf80201 acked 2026-10-05)
  new: O001 x2 (warning; error under --release), O003 airflow_ok (error)
commercial deltas (module = fan + bracket + grommet + 4 screws)
  unit cost     9.046 -> 7.122 USD   (-1.924, -21.3%)   [fan -2.000, screws -0.024]
  fan lead time 6wk -> 10wk (+4wk; program max 8wk)
  fan moq       50 -> 100
  module mass   180.6g -> 155.6g (-25.0g)
note: a 20mm fan passes the require if max_airflow >= ~43m^3/h at 22Pa (hand solve of the same curve).
```

## 3. `cadetta impact --swap acme.cooling::CoolingModule/fan acme.cooling::NoctuaNfA8Pwm` (EOL and performance pressure)

```
swap CoolingModule/fan: SunonMf80251 (eol 2027-09) -> NoctuaNfA8Pwm
verdict: PROMOTABLE with warnings (0 violated, 1 undecided)

values that change (direct)
  fan.mpn MF80251V1-1000U-A99 -> NF-A8 PWM       fan.supplier Sunon -> Noctua
  fan.inlet_dia 76 -> 74mm      fan.max_airflow 60 -> 90.9 m^3/h   fan.max_static 40 -> 23.2Pa
  fan.lead_len 300 -> 200 +-10mm   fan.connector_env 8 -> 11.4 +-0.4mm
  fan.mass 85g -> 78g   fan.unit_cost 6.85 -> 16.90USD   fan.lead_time 6wk -> 2wk   fan.moq 50 -> 1
  (unchanged: depth, width, hole_pitch, mount_hole, voltage)
values that change (derived)
  FanBracket.cutout_dia   77mm -> 75mm     FanBracket.guard_ring_id  77mm -> 75mm
  operating_flow(fan)     32.92 -> 30.43 m^3/h; margin to 24.1: +8.82 -> +6.33
  unchanged derived: screw_stack, screw_length 32mm, plate_w, pitch, screw cost
checks and fits that flip
  grommet_pass  holds -> UNDECIDED (connector [11.0,11.8] vs bore [11.3,11.5]) C002
  unchanged verdicts: airflow_ok holds, screw_pass holds, lead_time_ok holds (6wk -> 2wk),
    lead_reach_ok holds (margin 120mm -> 20mm), keepout_ok, tip_ok, moq_ok, others
alternates that become primary: NoctuaNfA8Pwm; resulting alt list [SunonMf80201]
  lifecycle: L001 cleared (primary now active)
CAD files to re-sync
  cad/freecad/fan_bracket.FCStd   cutout_dia 77->75mm
  cad/freecad/fan_guard.FCStd     guard_ring_id 77->75mm
  build/bom.csv                   (emit)
  not stale: cooling_module.FCStd (fan.depth and screw length unchanged)
attestations invalidated
  tolerance_review@2026-10-08  stale (grommet_pass now undecided; 9 of 12 nodes read the fan)
  thermal_signoff@2026-10-08   stale (airflow_ok reads the fan); requirement still holds, re-ack discharges
  source: none (NoctuaNfA8Pwm acked 2026-10-05)
commercial deltas
  unit cost     9.046 -> 19.096 USD  (+10.050, +111.1%)   [fan +10.050, screws 0]
  fan lead time 6wk -> 2wk (-4wk)    fan moq 50 -> 1
  module mass   180.6g -> 173.6g (-7.0g)
```

## 4. Can obligations.md 5 specify this deterministically?

No. The semantics of each answer are mostly derivable, but the following
are not fixed, and I had to choose:

1. Digest scope (ir.md 4 vs obligations.md 2). A check digest hashes its
   own expression; "stale when anything it reads changed" needs a
   transitive (Merkle) digest. The attestation-invalidated list is empty
   or 9 of 12 nodes depending on this. Also member-level vs instance-level
   granularity (does `sheet_ok` read the `screws` instance, whose
   argument changed, or only the constant bound?).
2. Derived values per alternate. The IR stores verdicts per alternate
   but one `value` per member. `screw_length` and `cutout_dia` differ per
   alternate; the preview needs a variant-parameterized evaluation that
   is not defined (also needed for `compare`'s "which checks hold").
3. Viability and promotion: is an undecided alternate promotable? How is
   a waiver scoped to one alternate? How is a violated `require` handled
   (waive, attest, or hard block)?
4. Swap list semantics: NEW is already in `alt`; old primary kept or
   dropped; `PART` path form for an interface-typed part.
5. Output schema and ordering: no JSON schema, units for deltas, how
   "mass" and "lead time" roll up across parts lacking them (`mass(self)`
   errors; no `max` roll-up exists for lead time), percentage rounding.
6. CAD resync rule: "bound param's emitted value changed" is clear, but
   which file a binding targets comes from `read` realizes edges and the
   lock; the source alone cannot tell (`#[native]` has no file).
7. Lifecycle date meaning (last-time-buy, EOL, verified-on) and how an
   announced future EOL on an `eol`-status part differs from `active`.
