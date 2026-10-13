# 07 part swap

An 80mm axial fan cooling module: a sheet-metal fan bracket, four ST4.2
self-tapping screws and a cable grommet, with one interface-typed slot
`CoolingModule/fan` (primary Sunon MF80251, end of life 2027-09; approved
alternates Noctua NF-A8 and Sunon MF80201). A thermal solver gives the
required airflow, and a `require` compares it with the fan's operating
point against a square-law system curve.

Exercises: variant evaluation and I002/I003 (language.md 3.2), roll-ups
and instance paths (language.md 3.1, 3.5), `std::commercial::Sourced`
names (stdlib.md 6), affine tolerances in fits and checks (quantities.md
5), variant solver results (solvers.md 4.1), attestation closure
(obligations.md 2, ir.md 4), and `impact --swap` (obligations.md 5.1,
adapters.md 3 bindings).

- Sunon MF80201 is blocked (require `airflow_ok` and `lead_time_ok`
  violate, `screw_pass` undecided): I002. Noctua is viable with
  `grommet_pass` undecided: I003. L001 is not raised.
- `expected/swap.json` is the blocked swap; `expected/swap-noctua.json`
  the viable one. Key names and order follow obligations.md 5.1.
- Attestation counts (tolerance_review 9 of 13, thermal_signoff 1 of 1)
  assume both roles are attested; the case ships no lock file because
  lock digests are computed by the implementation.
- `analysis/airflow_req.out.json` follows `cadetta.solve-result/1`
  with `blake3:TBD` digests, filled when the case is confirmed. Datasheet and CAD files are not
  shipped (S002 and R008 are info).
