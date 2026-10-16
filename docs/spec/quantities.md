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
  different `k` is inexact unless one of them is zero: `1deg + 1rad` is an
inexact value with a Q006 root (`mixed pi factors`), not an evaluation
error; only affine (toleranced) forms reject it (5.4). `floor`, `ceil`
and `round` of a value carrying a pi factor are likewise inexact with a
Q006 root, since the integer result is not exact.
- **Dimension** is an integer exponent vector over the seven SI base
  dimensions `[L, M, T, I, Theta, N, J]`, plus one base dimension per
  currency used (3).
- **Angle** is an integer angle exponent tracked beside the dimension.
  It does not affect the magnitude (`1rad` is 1) but it is checked (4.3).
- **Kind** names what the quantity means and refines dimension (4).
- **Tolerance** is an affine form over source variables (5.4); for a
  literal it is a pair of signed deviations `lower <= upper`.

A zero magnitude is normalised to `k = 0`, so equal magnitudes compare
structurally. A currency's exponent is dropped from the vector when it
reaches zero. Division by zero is an error value, never a panic.

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
- `5 mm` (whitespace before the unit) is Q007 with a fix-it. The lexer flags a built-in
  unit atom after spaces or tabs following a number (`5 in` is always
  this mistake); the fix-it deletes the whitespace and the joined
  literal is still emitted. `%` is excluded because it may be an operator.
- A fraction or mixed number is a literal only when a unit follows it
  immediately; a bare `3/4` is number, `/`, number, and a bare `1-1/2`
  is a subtraction. Literal numbers stay exact source text until sema
  converts them to rationals.
- Compound-literal order is judged by built-in unit size within Length,
  Time, Angle or Mass; any other unit there is a lex error (E001).
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
also carry an exact offset. Affine units cannot be multiplied or
divided (Q002, decided in the conformance fixtures), including a Temperature held in a variable or parameter; subtracting two Temperatures yields a TemperatureDelta.

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
   A package `kind Name = Expr` declaration registers `Name` as a named kind of that
   dimension, so it follows this rule like a built-in kind.
5. Assignment and `as` give the value the target kind's canonical angle
   exponent (4.3), after any Q008 the assignment raises. `as` may change the angle exponent but never the
   dimension: `travel as Length` turns `lead * 1rev`-style values into a
   plain Length without the `/ 1rad` idiom.

6. Decided rules (implementation, cadetta-units `kind.rs`):
   - A "dimensionless value" in rule 3 has dimensionless dimension and
     angle exponent 0 (Ratio, Count, anonymous). `Angle * Ratio` keeps
     Angle; `Angle * Angle` is anonymous.
   - A Count scaled by a non-Count dimensionless value is a Ratio
     (`Count * Count` stays Count; the product is not an integer
     otherwise). `Count / Count` is anonymous.
   - Table rows match operand kinds by name, not dimension: a
     package kind such as Stress does not match a Pressure row. A row
     applies only when the result dimension and angle equal the named
     kind's canonical ones; otherwise the result is anonymous.
   - `x ^ n` for `n` other than 2 or 3 on Length is anonymous.
   - `sqrt`/`cbrt` need every dimension exponent and the angle
     exponent to divide; otherwise Q011.
   - `as` never raises Q008 (it is the explicit angle conversion);
     it still requires equal dimension (Q002).
   - Torque and Energy share a dimension and differ only in angle
     exponent, so Torque assigned to Energy is Q003, while Torque
     against Length is Q002.
   - A literal takes its kind's canonical angle exponent, so
     `0.45N*m` already carries -1.
   - `floor`/`ceil`/`round` of a value carrying a `pi` factor are
     Q006 (not exactly representable as a Count).

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

### 4.4 Checker decisions

Fixed by the kind checker (`cadetta-sema` `kindcheck`); they refine, not
change, 2 and 4.1-4.3.

- An untyped member assigned a bare number is an anonymous dimensionless
  value; Q001 needs a declared dimensioned target.
- A bare number beside a dimensioned operand of `+ - min max`, a
  comparison or a range is Q001 at the bare operand.
- In `*` and `/` an integer bare number acts as a Count, so it never
  turns a Count into a Ratio; other bare numbers are anonymous scalars.
- `x ^ n` with a dimensioned base needs an integer literal `n`; any
  other exponent is Q002.
- A block `let` whose type is unknown shadows a same-named component
  member: reads of that name inside the block are untyped, never the
  member's dimension.
- `sin cos tan` take an Angle: a bare argument is Q001, any other
  dimension Q002, a wrong angle exponent Q008. They return Ratio.
- A tolerance deviation of a dimensioned nominal must have its
  dimension; a bare `0` is allowed. A bare nonzero deviation is Q001
  (at the deviation), a deviation of another dimension Q002 (decided in
  the conformance fixtures).
- An unknown kind or unit name is E002 (unresolved-name).
- A name that is not a local of the component being checked (`Other.width`,
  `pkg::Mod::Other.width`, an own member) is typed from its evaluated
  dimension, as an anonymous kind: a dimension mismatch against it is Q002,
  but a named-kind mismatch between two kinds of one dimension is not seen.
- Expressions that are not quantities (booleans, strings, arrays) and
  names the checker cannot type are unknown and never cascade.

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
holes [position 0.2mm]           // true position zone on Point2 values (geometry.md 3.1)
```

A bracket whose content begins with `+` or `-` is a tolerance; array
indexing never does (indices are non-negative). A bracket beginning with
the contextual keyword `position` is a position tolerance, legal only on
`Point2` values (geometry.md 3.1). Deviations must match the
nominal's dimension; units may differ (`1in [+-0.1mm]`). ISO 286 classes
resolve through `std::fits::iso286`; uppercase letters are holes, lowercase
shafts. F002 when the size is outside the table.

### 5.2 Accessors

Every quantity has `.nominal .min .max .mid` and, when it has a role
(5.6), `.mmc .lmc`. Each returns an untoleranced quantity. On an
untoleranced quantity all of them equal the value.

Decisions (`cadetta-units`): an untoleranced quantity is a toleranced one
with zero deviations, so the equality above is structural. `.mmc` is the
smallest size of a hole and the largest of a shaft; `.lmc` is the opposite.
An accessor that would add different nonzero powers of pi fails rather than
rounding (section 1).

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

Decided in implementation (`cadetta-sema` eval): the nominal of an
expression is tracked through arithmetic separately from the affine
centre, so `cb = head_dia + 1mm` stores the sum of nominals even when
`head_dia` is asymmetric; `.max` and the other accessors read bounds
and the tracked nominal. A parameter is an alias only when it is a pure
reference to a non-`let` quantity; a parameter reading a `let` stores
that `let`'s nominal untoleranced. A bracket with a single `+v` term has
0 on the lower side and a single `-v` term has 0 on the upper side; with
more terms the lower and upper deviations are the least and greatest
signed deviation. A tolerance written inside a larger expression or a
`let` owns an anonymous source variable. Evaluation failures with no
spec code (division by a divisor containing zero, mixed `pi` exponents,
a construct evaluated by a later stage) carry the internal code `EVAL`.
Parameterized components are evaluated only as instances.

Standard functions (language.md 3.8) follow these rules. An inexact value
(`sqrt` of a non-square, `cbrt` of a non-cube, `sin`, `cos`, `tan`, `atan2` off the special
angles) is carried as an `f64` nominal with an outward-widened
conservative interval and its root operation (Q006). It supports
`+ - * /`, negation, `min`, `max`, `abs` and `round_to`, `ceil_to`,
`floor_to`; any other use as a quantity is an `EVAL` error telling the
author to round it first. `sqrt` needs a dimension whose exponents are
all even (Q011) and is exact only for an exact form and exact value.
Trigonometric arguments must be angles (Q002); `tan` at a pole and
`atan2(0, 0)` are `EVAL`. `min` and `max` return a certain winner
unchanged (keeping its source variables), otherwise the interval hull
under a fresh variable. `floor`, `ceil`, `round` take dimensionless
values; `round` is half away from zero. `count` returns the length of
an array. `fmt` substitutes `{}` positionally (`{{` and `}}` escape);
a placeholder/argument mismatch is E006. A quantity renders in its
authored unit when one is syntactically visible, else in SI base units;
attribute-driven display is applied by the output stages. Recursion,
direct or mutual, is rejected with `EVAL` at the function's name. A
non-exhaustive `match` is E009 naming each missing variant.

### 5.4 Evaluation is affine

Each toleranced parameter of each part instance is one independent
source variable; aliases share their referent's variable (5.3). Expressions carry tolerance as an affine form `nominal
+ sum(c_i * e_i)` over those variables, so repeated or cancelling terms
are counted once and linear chains are exact. Nonlinear operations
(products of two toleranced values, roots, trig) fall back to interval
bounds and mark the result conservative; a conservative `undecided`
says so in its C002 message. `* /` of two toleranced quantities is
legal and conservative (it was Q005 in draft 0; Q005 is retired).
Each source variable ranges over `[-1, 1]` and an asymmetric literal
`[lower upper]` contributes centre `nominal + (lower + upper)/2` and
coefficient `(upper - lower)/2`; a nonlinear fallback keeps the interval
hull as a centre plus a variable-free symmetric slack, which only grows.
Adding terms of different nonzero `pi^k` is an error in a form (it would
be inexact); `sqrt`, `sin`, `cos` and `tan` return an outward-widened
binary64 interval, always conservative, and `tan` over a pole is a
domain error.
`x ^ 2` and `x * x` of the same affine form use a square-aware
interval: the result is never negative, so `sqrt(x^2 + y^2)` (and
`radial`, language.md 3.8) stays defined when x straddles zero.

A position tolerance (geometry.md 3.1) adds a disc source variable: a
pair `(u, v)` constrained by `u^2 + v^2 <= 1` rather than each in
`[-1, 1]`, entering a point's x and y with coefficients `d/2`. The pair
appears in forms as two terms that share the variable's path with the
suffixes `.u` and `.v`. A bound over a form groups a disc's two terms
and adds `sqrt(a^2 + b^2)` for the coefficients `a` of `u` and `b` of
`v`, instead of `|a| + |b|`; the square root is exact when either coefficient is
zero and otherwise outward-widened and conservative. Treating the pair
as two independent terms is never unsound, only looser.

Decided in implementation: "the same affine form" means structurally
equal forms (same centre, terms and slack). `x * -x` and `x * (x + 0)`
with differing slack are not detected and use the ordinary product. The
square of an exact form is exact; of a toleranced one it is the interval
hull of `{a^2, b^2}` (with lower bound 0 when `[a, b]` contains zero),
marked conservative.

Decided in implementation (`cadetta-sema` affine): source variables are
keyed by (instance path, parameter name), so two instances of one part
own distinct variables and vary independently (OQ-9 stays open for
matched lots). An alias binds to its referent's variable and never
allocates; a tolerance written inside an expression owns an anonymous
variable. C004 is recorded when a stored parameter is computed (no
variable, not an alias, not a `let`) while the form of its expression
carries tolerance.

### 5.5 Rounding

`round_to(x, step)`, `ceil_to(x, step)`, `floor_to(x, step)`; `step`
must have x's dimension. The result is an exact rational multiple of
step. Ties round away from zero. The nominal is rounded; deviations are
kept. If x is inexact and within 1e-9 (relative) of a tie or step
boundary, the result is Q010 (error: ambiguous rounding).

Decided in implementation: the boundary is a tie for `round_to` and an
integer multiple of step for `ceil_to` and `floor_to`; "within 1e-9
(relative)" means a distance, in step units, of at most `1e-9 *
max(|boundary|, 1)`. A quotient `x / step` with differing `pi`
exponents is irrational, so it is treated as inexact for this check.
`sqrt` is exact for a perfect-square rational with an even `pi`
exponent (`sqrt(4mm^2)` is exactly `2mm`) and inexact otherwise; `cbrt`
likewise for a perfect cube with a `pi` exponent divisible by 3 (negative
values allowed), and a toleranced operand gets an interval cube root; an
inexact operand keeps its own root operation, so Q006 names the first
one. `tan` at an odd multiple of 90deg and `atan2(0, 0)` are domain
errors.

### 5.6 Roles and the emitted point

A role (`hole` or `shaft`) comes from an ISO class's letter case, from
the side a parameter plays in a `fit`, or from `#[role(hole)]`; pure
references keep it. The point written to a CAD tool is chosen by policy:
`nominal` (default), `mid`, `mmc`, `lmc`, set by `[emit] point` and
overridden per parameter with `#[emit(mid)]` or for a scope with
`#![emit(mid)]`. `mmc`/`lmc` without a role is E010. A pure
reference inherits its referent's emit point unless it sets its own, so
an alias of a `#[emit(mid)]` fit side is emitted at mid too.

The point is chosen on the designed value (`Value.nominal`), then the
member's process rule, if any, is applied to that point (language.md
3.8). The result is what is emitted, what `drawn?` holds in the IR
(ir.md 3) and what R001 compares against. The drawn value carries no
tolerance of its own.

### 5.7 Bounds

A bound is a one-sided or two-sided requirement, not a value (language.md
3.4): `head_dia <= 8.5mm`, `abutment_dia >= 9.6mm`, `x in 4mm..6mm`.
`x satisfies P` in a check tests x against bound P; `P.min`/`P.max`
read its limits. A bound is never emitted.

Decisions: `x <= v` has only a `.max` and `x >= v` only a `.min`; reading the
absent limit yields no value (the checker reports it), never an infinity.
`x in lo..hi` requires `lo <= hi`. Role provenance is recorded (ISO class,
fit side, attribute, inherited); an explicit role on a reference wins over
the inherited one.

A fit gives its operands a role that `mmc`/`lmc` use when nothing stronger
says otherwise: in `a / b` the first path is the hole and the second the
shaft; in a class fit (`p6 on x`) the `on` member plays the mate of the
class letter. Precedence: `#[role]` attribute, then the member's own ISO
class, then the fit side. A class pair (`H7/p6`) names one member for both
sides and implies no role.

### 5.8 Comparisons are three-valued

`holds` (true for every value in tolerance), `violates` (false for
every value) or `undecided` (C002). Exact comparisons are exact. When an
operand is inexact and the difference is within 1e-9 relative, the
result is `undecided` with an inexactness note. `==` on toleranced
values compares nominals only when written on accessors (`a.nominal ==
b.nominal`); on raw toleranced values it is almost always `undecided`
and C003 suggests the accessor form.

Decision table, with `d = a - b` taken symbolically so shared source
variables cancel (5.4): for `>`, `holds` if `min d > 0`, `violates` if
`max d <= 0`; `>=`, `<`, `<=` are the mirror cases; `==` holds only when
`d` is exactly zero and violates when zero is outside `d`; `!=` is its
negation. When an operand is inexact or the operands carry different
nonzero `pi^k`, `d` is computed in binary64 and widened by 1e-9 of the
largest operand magnitude on each side before deciding. C003 is raised
for any `==`/`!=` with a toleranced operand, in addition to C002 when
the result is undecided. An inexact operand (`sqrt`, trigonometry) is
accepted in a `check`/`require` comparison as such an interval, so
`sqrt(2mm^2) >= 1.4142135623731mm` is undecided (C002) while
`>= 1mm` holds.

## 6. Display

Storage is SI; display unit selection, in order: `#[display(unit)]` on
the parameter, `#![display(...)]` on an enclosing scope, `[units]
display` in `cadetta.toml`, then the authored unit. Fractional displays:
`#[display(in/32)]` prints `45-15/16in`; `#[display(ft_in)]` prints
`3'9-15/16"`. The formatter preserves authored literals.

Display rules (decided 2026-10-13, cadetta-units `display`):

- A trailing `/<integer>` makes a fraction spec (`in/32`); anything else
  is a unit expression (`mm/rev` is a quotient). The denominator is at
  least 1. `ft_in` alone means `ft_in/16`; `ft_in/N` is accepted.
- The value is rounded once to the nearest `1/N` of the unit, ties away
  from zero, and the fraction prints in lowest terms: `45.9375in` at
  `in/32` is `45-15/16in`. A zero fraction is dropped (`2in`), a zero
  whole before a fraction is dropped (`1/2in`), and `ft_in` carries
  into feet (`11.99in` is `1'0"`).
- `[units] display` supplies one spec to a value (the caller looks it up
  by kind); the display unit must have the value's dimension.
- Plain units print the exact decimal when it terminates, otherwise six
  places, half away from zero, trailing zeros trimmed. Money prints at
  the currency minor unit (ISO 4217 digits: 2, but 0 for JPY and 3 for
  KWD, for example) and `%` prints one decimal, both half away from
  zero (obligations.md 5.1). Fractional specs need a length unit.

Conformance surface (decided 2026-10-16): `cadetta display VALUE SPEC`
prints one quantity through the formatter (config-cli.md 3.3). `VALUE` is
a quantity literal (`45.9375in`, `0.25m`; a bare number is dimensionless)
and `SPEC` is the text inside `#[display(...)]`. The `kind = "binding"`
fixtures under `tests/conformance/quantities/` are
`{ "call": "display", "value", "spec" }` JSON; the runner invokes the verb
and compares its printed text with the expected `result`. A separate verb
(not `units display`) was chosen because `units` lists tables keyed by
KIND while `display` is a pure computation on a value and spec: the
arguments map one to one onto the fixture call and the future
`cadetta.display(value, spec)` binding, and it needs no project.

## 7. Diagnostics owned by this document

Q001 unitless-quantity, Q002 dimension-mismatch, Q003 kind-mismatch,
Q004 inexact-emit, Q006 inexact-value, Q007 quantity-literal-split, Q008
angle-dropped, Q009 non-integer-count, Q010 ambiguous-rounding, C003
toleranced-equality, C004 derived-feature-in-check. Q005 is retired.

One fault yields one diagnostic: a lexical error is not followed by a parse
error on the same line, and an evaluator report that overlaps a kind-check
diagnostic in the same module is dropped. `check` and `require` conditions are
kind-checked like any expression (`x >= 5` on a length is Q001).

## Open questions

- OQ-1 (closed 2026-10-13, README): `5.in` replaced by `5in`; README rewritten.
- OQ-2: Statistical (RSS) tolerance arithmetic. Deferred; the affine form
  (5.4) is the right substrate for it later.
- OQ-3 (closed 2026-10-13, language.md 3.10): packages may declare
  kinds of an existing dimension (`kind Stress = Pressure`).
- OQ-9: Per-instance source variables (5.4) treat two instances of one
  part as varying independently. Is a "matched set" (same lot) mode
  needed? Not in 0.1.0: pilot r2-02 needed it for a bearing pair and
  wrote the independent (conservative) analysis instead.
