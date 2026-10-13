# Friction report: 02 bearing housing (MEDIUM)

## 1. Summary
Plain dimensions, ISO fits on designed features, and the three-valued
checks worked well. The language broke on the point the pilot targeted:
fits against an OTS part that has its own non-ISO tolerance. The literal
syntax for asymmetric tolerance is ambiguous with arithmetic, naive
interval arithmetic produces false C002s on correlated terms, and there
is no way to separate a vendor tolerance from a designed one.

## 2. Findings
F1 blocker - language.md 3.2, quantities.md 5.2. Wanted: shaft j5 on a bearing bore of `8 +0 -0.008`. `fit` needs HOLE/SHAFT, so I must invent an H class (F003 vs vendor tolerance, and a fictional hole corrupts fit_type). Workaround: authored `8mm j5` plus a computed `seat_interference`; the fit is lost from the IR. Change: `fit bearing_seat = j5 on Bearing.bore` (hole side taken from the tolerance on the path; F003 not applicable).
F2 blocker - quantities.md 2, 5. `bore = 8mm +0mm -0.008mm` also parses as `8mm + 0mm - 0.008mm` = 7.992mm. `10mm +0.05mm -0.02mm` has the same lexical clash with binary +/-. Change: bracket the deviations: `8mm [+0mm -0.008mm]`, and state `+-`/bracket precedence vs `a + b +- c`.
F3 awkward - quantities.md 5.1, language.md 5. Tolerances propagate from vendor into designed parts: `Shaft.length` becomes `67 +0 -0.12` because of bearing width; `Spacer.length` is `23 +0.12 -0`. No way to take a nominal or limit. Also `8mm j5` restates the bore (D001). Change: accessors `Bearing.bore.nominal`, `.min`, `.max`; `Bearing.bore.nominal j5`.
F4 awkward - quantities.md 4 rule 2, 5.1. Interval arithmetic treats repeated terms as independent. `axial_play` contains `Bearing.width` via `Spacer.length = S - B` plus `2B`, which cancel; the spec yields [-0.07, 0.59] mm instead of [0.05, 0.47], so both checks are undecided (false C002). Change: evaluate in affine form (shared noise symbols per source parameter) or simplify before evaluation; document that C002 may be conservative.
F5 awkward - quantities.md 5.2. `@emit(mmc)` needs hole vs shaft. Explicit-tolerance literals (`8 +0 -0.008`) have no role, and it is unstated whether the role survives a reference (`Housing.groove_dia = Circlip.groove_dia`, an H11 internal groove) or arithmetic (Spacer). Change: role tag `@role(hole|shaft)` inherited through pure references.
F6 awkward - language.md 3.1, lints M003/D001. Limit values (d_a min, D_a max, clearance range) and recommendations (DIN 472 groove) are not "facts about the part" and carry no bound semantics. Change: `abutment_dia_min: Length >= 10mm`-style bounded params, which M003 exempts; allow `ots` to hold `recommend` blocks.
F7 awkward - language.md 6, quantities.md 5.3. `==` on toleranced values is always undecided, so "ring nominal equals bore nominal" is unwritable; I used `<=`. Change: `check ring_size_ok: nominal(ring.bore_nominal) == nominal(housing.bore_dia)`.
F8 nit - language.md 3, 4. `import ... as` has no use syntax; path grammar for an imported class (`Bearing` vs `acme.vendor::Bearing`) is unspecified, and class paths (`Bearing.width`) and instance paths (`bearing_a.width`) mix. No global params, so I made a `Layout` model.
F9 nit - language.md 6. No quantifier: `bearing_b` is never checked; copy-paste only.
F10 nit - language.md 3.2. F003 unspecified: designation match or numeric match? Bearing OD 0/-9 equals h5 numerically only by coincidence of IT5.

## 3. Hypothetical `cadetta check`
- C002 seat_creep (j5 may be loose, [-2,+12]um): useful.
- C002 seat_squeeze ([-1.6,+9.6]um vs 2um min clearance): useful, real design concern (consider C3).
- C002 axial_play_min, axial_play_max: wrong (F4).
- D001 Circlip.bore_nominal vs Bearing.outer_dia (22mm): noise, ots cannot reference.
- D001 Shaft.seat_dia: waived; if D001 compares tolerance-free nominal only it does not fire and W004 results (ambiguous).
- D002 Housing.groove_pos (40 = 3+23+14): wrong, deliberately independent.
- M003 on ~7 limit params (clearance_min, abutment_*, groove_edge_min, bore_nominal, end_margin): noise.
- No F-codes: housing_seat passes F003.
Uncaught by any lint: bearing_b unchecked; interference sign flipped (`bore - shaft`) silently inverts meaning; inner ring axial retention missing; circlip size relates to bore only by a loose check; bearing load rating vs application; wrong class choice (k5 vs j5) is judgment.

## 4. Evaluation trace (mm unless noted)
a) seat_interference = 8 j5 [-2,+4]um minus bore [-8,0]um: nominal 0; lower -2-0 = -2um; upper +4-(-8) = +12um. Rational (SI): nominal 0/1, lower -1/500000, upper 3/250000. Ambiguity: IR `lower/upper` are deviations or absolute limits? For nominal 0 both read the same, so the field must be defined.
b) Shaft.length = 22+30+7+8 = 67; deviations 0+0+[-0.12,0]+0 = [-0.12,0]. Emitted at nominal: 67.
c) axial_play naive: Sum1 = 40 +-0.05 + [1.1,1.24] - [0.94,1.0] = 40.1 [-0.05,+0.25]; Sum2 = 3 + 23[0,0.12] + 14[-0.24,0] = 40 [-0.24,+0.12]; difference 0.1 [-0.17,+0.49]. Correlated truth: Sum2 = 33+B = 40 [-0.12,0] gives 0.1 [-0.05,+0.37]. Ambiguity: `2B` written as `bearing_a.width + bearing_b.width` (two instances of one class) -- are they the same variable or independent? Spec silent; independent is correct for physical parts, wrong for Spacer's derivation.
