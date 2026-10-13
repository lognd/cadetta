# Quantities

Every value in cadetta is a quantity: an exact magnitude with a
dimension and a kind, optionally carrying a tolerance. There are no bare
floating-point parameters. This document is the foundation the others
build on.

## 1. Model

A quantity is `(magnitude, dimension, angle, kind, tolerance?)`.

- **Magnitude** is stored in SI base units as `r * pi^k`: `r` an exact
  rational of arbitrary precision, `k` an integer. `1in` is exactly
  `127/5000 m`; `1deg` is exactly `1/180 * pi^1 rad`; pi cancels exactly
  (`360deg / 1.8deg` is exactly `200`). Exactness is preserved through
  `+ - * /`, integer powers and rounding. Adding two magnitudes with
  different `k` is inexact unless one of them is zero.
- **Dimension** is an integer exponent vector over the seven SI base
  dimensions `[L, M, T, I, Theta, N, J]`, plus one base dimension per
  currency used (3).
- **Angle** is an integer angle exponent tracked beside the dimension.
  It does not affect the magnitude (`1rad` is 1) but it is checked (4.3).
- **Kind** names what the quantity means and refines dimension (4).
- **Tolerance** is an affine form over source variables (5.4); for a
  literal it is a pair of signed deviations `lower <= upper`.

Inexact operations (trigonometry, roots, non-integer powers, addition
across `pi^k`) yield a quantity marked inexact, carried as an IEEE-754
binary64. Rounding (5.5) restores exactness.

## 2. Literals

A quantity literal is ONE token: a number immediately followed by a unit.

```
5in   12.7mm   90deg   0.25rev   3.2N*m   9.81m/s^2   1500rpm   8mm/rev
5"    2'       3/4in   1-1/2in   1/16"    2'6"      2ft6in      1h30min
```

- **Numbers**: integers `12`, decimals `12.5`, exponents `1e-3`,
  fractions `3/4`, mixed numbers `1-1/2`. A leading or trailing `.` is a
  lex error. A fraction or mixed number is part of the token only when no
  whitespace appears inside it: `3/4in` is three quarters of an inch;
  `3 / 4in` is three divided by four inches.
- **Units**: unit atoms joined by `*` and `/`, each with an optional
  integer exponent `^n` (`^-n` allowed). No spaces, no parentheses.
- **Quote aliases**: `"` and `'` directly after a number are `in` and
  `ft`. `5"abc"` is a lex error, by design.
- **Compound literals**: two or more number-unit segments of the same
  dimension in strictly descending unit size, no spaces: `2'6"`,
  `2ft6in`, `1h30min`. Only the last segment may be a decimal.
- `5 mm` (whitespace before the unit) is Q007 with a fix-it.
- **Bare numbers** are dimensionless and of anonymous kind. They are
  legal (a) as factors, divisors or exponents, (b) as operands of `+ -`
  and comparisons whose other operand is dimensionless (a Count, a Ratio
  or an anonymous dimensionless value such as `x / step`), and (c) as the
  value of a Count or Ratio parameter. Anywhere else: Q001.
- `%` is a unit: `5%` is exactly `1/20` of kind Ratio.

## 3. Units

Built-in units (normative; packages may add units and kinds with `unit`
and `kind` declarations, language.md 3.10):

| Kind (default for the unit) | Units |
|---|---|
| Length | `m mm cm um nm km in ft yd mil thou` |
| Angle | `rad deg arcmin arcsec rev` (angle exponent 1) |
| Area / Volume | any Length unit `^2` / `^3`; `L mL` |
| Mass | `kg g mg lb oz` (`lb` is pound-mass) |
| Force | `N kN lbf` |
| Torque | `N*m N*mm lbf*in lbf*ft` |
| Energy | `J kJ Wh` |
| Power | `W kW hp` |
| Pressure | `Pa kPa MPa GPa psi ksi bar mmH2O inH2O` |
| Time | `s ms min h d wk` |
| Money | `USD EUR GBP CNY JPY ...` (ISO 4217; each currency is its own base dimension, never converted implicitly: exchange rates are data, not constants) |
| Frequency | `Hz kHz` |
| Velocity | `m/s mm/s in/s ft/min` |
| VolumeFlow | `m^3/s m^3/h L/min cfm` |
| Acceleration | `m/s^2 gn` |
| AngularVelocity | `rad/s rpm deg/s` (angle exponent 1) |
| Lead | `mm/rev in/rev` (Length per Angle) |
| Current / Voltage / Resistance | `A mA` / `V mV` / `ohm kohm` |
| Inertia | `kg*m^2` |
| Density | `kg/m^3 g/cm^3 lb/in^3 lb/ft^3` |
| Temperature | `K degC degF` (absolute; affine) |
| TemperatureDelta | `dK ddegC ddegF` |
| Ratio | `%` and bare numbers |
| Count | bare integers |

Every unit has an exact factor `r * pi^k` to SI; affine temperature units
also carry an exact offset. Affine units cannot be multiplied;
subtracting two Temperatures yields a TemperatureDelta.

## 4. Kinds

### 4.1 Assignment and arithmetic

1. A literal gets its unit's default kind. A compound unit not in the
   table gets an anonymous kind of its dimension and angle.
2. `+ - min max` and comparisons need equal kinds, or a named kind and
   an anonymous kind of equal dimension and angle; the result is named.
3. `*` and `/` yield an anonymous kind, except: a named kind scaled by a
   dimensionless value keeps its kind (`thickness / 2` is a Length);
   `Count * Count` is Count; the kind algebra table below names the
   rest. Force*Length is deliberately not named.
   `x ^ n` takes an integer literal `n` and multiplies the dimension by
   it. `sqrt` and `cbrt` divide the dimension by 2 and 3; a dimension
   that does not divide is Q011. The result kind is then looked up in
   the table, else anonymous.

   | Product or quotient | Named kind |
   |---|---|
   | `Length * Length`, `Length ^ 2` | Area |
   | `Area * Length`, `Length ^ 3` | Volume |
   | `sqrt(Area)`, `cbrt(Volume)` | Length |
   | `Length / Time` | Velocity |
   | `Velocity / Time` | Acceleration |
   | `Mass * Acceleration` | Force |
   | `Force / Area` | Pressure |
   | `Mass / Volume` | Density |
   | `Volume / Time` | VolumeFlow |
   | `Energy / Time` | Power |
   | `Angle / Time` | AngularVelocity |

4. Anonymous kinds are assignable to any named kind of equal dimension
   and angle. A Count is assignable to a Ratio (never the reverse: that
   needs `floor`, `ceil` or `round`). Named kinds never convert implicitly: Q003. Explicit
   conversion: `x as Energy`.
5. Assignment and `as` give the value the target kind's canonical angle
   exponent (4.3), after any Q008 the assignment raises. `as` may change the angle exponent but never the
   dimension: `travel as Length` turns `lead * 1rev`-style values into a
   plain Length without the `/ 1rad` idiom.

### 4.2 Count

Count values are integers. A non-integer Count is Q009 (error). `count`,
array lengths and part arrays produce Counts, and so do `floor(x)`,
`ceil(x)` and `round(x)` on a dimensionless `x` (ties away from zero):
`teeth = floor(length / pitch) + 1` is a Count.

### 4.3 Angles

The angle exponent catches the missing-2*pi class of error. Magnitudes
are always computed with `1rad = 1`, which is what SI physics formulas
assume; the exponent only decides whether a diagnostic fires.

- Each named kind has a canonical angle exponent: Angle and
  AngularVelocity +1, Lead -1, Torque -1 (a torque is N*m per radian, so
  `Torque * Angle` is Energy and `Torque * AngularVelocity` is Power),
  everything else 0.
- Assigning or comparing a value whose angle exponent differs from the
  target kind's is Q008 (`angle-dropped`, warning). One exception: an
  exponent-0 value assigns to Torque, so a lever arm `F * r` is a Torque.
  Like every assignment (4.1 rule 5) it is then stored with Torque's
  exponent -1, so a later `torque / lead` is clean.
- Worked cases:

  | Expression | Exponent | Result |
  |---|---|---|
  | `1500rpm * 8mm/rev` to Velocity | 0 | exactly 200mm/s, clean |
  | `1500rpm * 8mm` to Velocity | +1 | Q008: lead declared as a Length; value off by 2*pi |
  | `0.45N*m * eta / 8mm/rev` to Force | 0 | 2*pi*T*eta/lead, clean, no `2*pi` written |
  | `0.45N*m * eta / 8mm` to Force | -1 | Q008: lead declared as a Length |
  | `50N * 9mm` to Torque | 0 | clean (lever-arm exception) |

- Trigonometric functions take Angle and return Ratio; `atan2(y, x)`
  takes two quantities of equal kind and returns Angle. They are exact
  where the result is rational in `pi`: `sin cos tan` of exact multiples
  of 90deg, `sin`/`cos` giving 1/2 and `tan` giving 1 at the matching
  multiples of 30deg and 45deg, and `atan2` when an argument is zero or
  `|y| == |x|` (so `atan2(3in, 3in) == 45deg` holds). Otherwise
  inexact (1.).

## 5. Tolerances

### 5.1 Syntax

Tolerance is a postfix operator binding tighter than any binary operator:

```
10mm [+-0.05mm]                  // symmetric
8mm [+0 -0.008mm]                // asymmetric, upper first; 0 needs no unit
12mm [-0.006mm -0.017mm]         // both deviations below nominal (a g6 shaft written out)
round_to(x, 1/32in) [+- Extrusion.cut_tol]   // any expression, any operand
(a + b) [+-0.1mm]                // parenthesize to tolerance a sum
20mm H7      20mm g6             // ISO 286 class, bare postfix
Bearing.bore.nominal j5          // class on any untoleranced expression
9.98mm..10.05mm                  // interval value; nominal is the midpoint
```

A bracket whose content begins with `+` or `-` is a tolerance; array
indexing never does (indices are non-negative). Deviations must match the
nominal's dimension; units may differ (`1in [+-0.1mm]`). ISO 286 classes
resolve through `std.fits.iso286`; uppercase letters are holes, lowercase
shafts. F002 when the size is outside the table.

### 5.2 Accessors

Every quantity has `.nominal .min .max .mid` and, when it has a role
(5.6), `.mmc .lmc`. Each returns an untoleranced quantity. On an
untoleranced quantity all of them equal the value.

### 5.3 Design tolerance vs analysis tolerance

**A parameter is a feature someone makes or buys; a `let` is an analysis
value. A parameter stores its design tolerance; a `let` keeps the full
analysis tolerance of its expression.**

- A parameter assigned from arithmetic is a feature: it is drawn,
  emitted and made to its own tolerance. It stores the nominal of the
  expression and only the tolerance written on it: `cb = head_dia.max +
  1mm` and `cb = head_dia + 1mm` both store an untoleranced 9.5mm. The
  feature is its own source variable (5.4); it does not vary with the
  values it was computed from, because once made it does not.
- A parameter that is a pure reference (`seat = fit.shaft`, `od =
  wheel.diameter`) is an alias: it is the referent's source variable,
  not a copy, so `housing.groove_width` and `ring.groove_width` vary
  together.
- A member `let` (`let axial_play = sum1 - sum2`, language.md 3.3) is
  an analysis value: never emitted, never bound, never a source
  variable. It keeps the affine form of its expression, so a check that
  reads it sees every tolerance upstream. Assembly-level stack-ups are
  written as `let`.
- A `check` evaluates its expressions over the full tolerance of every
  source variable they reach. Since each part's parameters hold that
  part's own drawing or vendor tolerance, this is the worst-case
  analysis of the assembly as built.
- A check that reads an arithmetic parameter with no tolerance of its
  own whose inputs are toleranced is C004 (info: "did you mean `let`?"),
  because the check then treats a computed value as exact.

### 5.4 Evaluation is affine

Each toleranced parameter of each part instance is one independent
source variable; aliases share their referent's variable (5.3). Expressions carry tolerance as an affine form `nominal
+ sum(c_i * e_i)` over those variables, so repeated or cancelling terms
are counted once and linear chains are exact. Nonlinear operations
(products of two toleranced values, roots, trig) fall back to interval
bounds and mark the result conservative; a conservative `undecided`
says so in its C002 message. `* /` of two toleranced quantities is
legal and conservative (it was Q005 in draft 0; Q005 is retired).

### 5.5 Rounding

`round_to(x, step)`, `ceil_to(x, step)`, `floor_to(x, step)`; `step`
must have x's dimension. The result is an exact rational multiple of
step. Ties round away from zero. The nominal is rounded; deviations are
kept. If x is inexact and within 1e-9 (relative) of a tie or step
boundary, the result is Q010 (error: ambiguous rounding).

### 5.6 Roles and the emitted point

A role (`hole` or `shaft`) comes from an ISO class's letter case, from
the side a parameter plays in a `fit`, or from `#[role(hole)]`; pure
references keep it. The point written to a CAD tool is chosen by policy:
`nominal` (default), `mid`, `mmc`, `lmc`, set by `[emit] point` and
overridden per parameter with `#[emit(mid)]` or for a scope with
`#![emit(mid)]`. `mmc`/`lmc` without a role is an error. A pure
reference inherits its referent's emit point unless it sets its own, so
an alias of a `#[emit(mid)]` fit side is emitted at mid too.

### 5.7 Bounds

A bound is a one-sided or two-sided requirement, not a value (language.md
3.4): `head_dia <= 8.5mm`, `abutment_dia >= 9.6mm`, `x in 4mm..6mm`.
`x satisfies P` in a check tests x against bound P; `P.min`/`P.max`
read its limits. A bound is never emitted.

### 5.8 Comparisons are three-valued

`holds` (true for every value in tolerance), `violates` (false for
every value) or `undecided` (C002). Exact comparisons are exact. When an
operand is inexact and the difference is within 1e-9 relative, the
result is `undecided` with an inexactness note. `==` on toleranced
values compares nominals only when written on accessors (`a.nominal ==
b.nominal`); on raw toleranced values it is almost always `undecided`
and C003 suggests the accessor form.

## 6. Display

Storage is SI; display unit selection, in order: `#[display(unit)]` on
the parameter, `#![display(...)]` on an enclosing scope, `[units]
display` in `cadetta.toml`, then the authored unit. Fractional displays:
`#[display(in/32)]` prints `45-15/16in`; `#[display(ft_in)]` prints
`3'9-15/16"`. The formatter preserves authored literals.

## 7. Diagnostics owned by this document

Q001 unitless-quantity, Q002 dimension-mismatch, Q003 kind-mismatch,
Q004 inexact-emit, Q006 inexact-value, Q007 quantity-literal-split, Q008
angle-dropped, Q009 non-integer-count, Q010 ambiguous-rounding, C003
toleranced-equality, C004 derived-feature-in-check. Q005 is retired.

## Open questions

- OQ-1 (closed 2026-10-13): `5.in` replaced by `5in`; README rewritten.
- OQ-2: Statistical (RSS) tolerance arithmetic. Deferred; the affine form
  (5.4) is the right substrate for it later.
- OQ-3 (closed 2026-10-13): packages may declare kinds of an existing
  dimension (`kind Stress = Pressure`), language.md 3.10.
- OQ-9: Per-instance source variables (5.4) treat two instances of one
  part as varying independently. Is a "matched set" (same lot) mode
  needed? Not in 0.1.0: pilot r2-02 needed it for a bearing pair and
  wrote the independent (conservative) analysis instead.
