# Quantities

Every value in cadetta is a quantity: a magnitude with a dimension and a
kind, optionally carrying a tolerance. There are no bare floating-point
parameters. This document is the foundation the other documents build on.

## 1. Model

A quantity is the tuple `(magnitude, dimension, kind, tolerance?)`.

- **Magnitude** is stored in SI base units as an exact rational number
  (arbitrary precision). `1in` is exactly `127/5000 m`. Exactness is
  preserved through `+ - * /` and integer powers.
- **Dimension** is an integer exponent vector over the seven SI base
  dimensions `[L, M, T, I, Theta, N, J]`. Two quantities can be added or
  compared only when their dimensions are equal.
- **Kind** names what the quantity means. Kinds refine dimensions: Torque
  and Energy share the dimension `M L^2 T^-2` but are different kinds;
  Angle, Ratio and Count are all dimensionless but different kinds.
- **Tolerance** is an optional pair of signed deviations
  `(lower, upper)` with `lower <= 0 <= upper` not required (a fit such as
  `p6` has both deviations positive) but `lower <= upper` required.

Inexact operations (`sqrt`, trigonometry, non-integer powers) yield a
quantity marked inexact, carried as an IEEE-754 binary64 alongside a
flag. Inexactness propagates. Emitters and lints MAY treat inexact values
differently (Q006).

## 2. Literals

A quantity literal is ONE token: a number immediately followed by a
unit, with no whitespace.

```
5in   12.7mm   90deg   0.25rev   3.2N*m   9.81m/s^2   1500rpm   5"   2'
```

- Numbers are decimal: `12`, `12.5`, `0.5`, `1e-3`. A leading `.` is not
  allowed (`.5mm` is a lex error); a trailing `.` is not allowed.
- A unit expression is unit atoms joined by `*` and `/`, each with an
  optional integer exponent `^n` (`^-n` allowed). No spaces, no parens.
- `"` and `'` directly after a number are aliases for `in` and `ft`.
  Strings elsewhere use `"`; the lexer disambiguates by adjacency to a
  number. `5"abc"` is therefore a lex error, by design.
- A number with whitespace before its unit (`5 mm`) is Q007
  (`quantity-literal-split`), an error with a fix-it.
- A bare number (`5`) is a dimensionless literal of anonymous kind. It is
  legal only (a) as a factor or exponent inside an expression (`2 * r`,
  `x^2`), or (b) as the value of a parameter declared `Count` or `Ratio`.
  Anywhere else it is Q001 (`unitless-quantity`), an error.

## 3. Units

The built-in unit table (normative; packages may add units):

| Kind (default for the unit) | Units |
|---|---|
| Length | `m mm cm um nm km in ft yd mil thou` |
| Angle | `rad deg arcmin arcsec rev` |
| Area | `m^2 mm^2 in^2` (any Length unit squared) |
| Volume | `m^3 mm^3 in^3 L mL` |
| Mass | `kg g mg lb oz` (`lb` is pound-mass) |
| Force | `N kN lbf` |
| Torque | `N*m N*mm lbf*in lbf*ft` |
| Energy | `J kJ` |
| Pressure | `Pa kPa MPa GPa psi ksi bar` |
| Time | `s ms min h` |
| Frequency | `Hz kHz` |
| AngularVelocity | `rad/s rpm` |
| Temperature | `K degC degF` (absolute; affine) |
| TemperatureDelta | `dK ddegC ddegF` |
| Density | `kg/m^3 g/cm^3` |
| Ratio | `%` and bare numbers |
| Count | bare integers |

Every unit has an exact rational factor to SI, except affine temperature
units which also have an exact offset. Affine units may not be
multiplied; subtracting two Temperatures yields a TemperatureDelta.

## 4. Kinds

A kind has a name and a dimension. Rules:

1. A literal gets the default kind of its unit (table above). A compound
   unit not in the table (`N*m*s`) gets an anonymous kind of its
   dimension.
2. `+`, `-`, `min`, `max` and comparisons require equal kinds, or one
   anonymous kind with an equal dimension; the result has the named kind.
3. `*` and `/` produce an anonymous kind, except where the kind algebra
   table names the result (Length*Length = Area, Length*Length*Length =
   Volume, Force*Length is deliberately NOT named, since it is Torque or
   Energy).
4. An anonymous kind is assignable to a parameter declared with any
   named kind of the same dimension. A named kind is never implicitly
   converted to another named kind: `x: Energy = some_torque` is Q003
   (`kind-mismatch`). Explicit conversion is `some_torque as Energy`.
5. Angle participates in `*` and `/` as dimensionless, valued in radians;
   `r * theta` yields an anonymous kind of dimension Length, assignable
   to a Length.

## 5. Tolerances

A toleranced quantity carries deviations from a nominal. Forms:

```
10mm +- 0.05mm             // symmetric
10mm +0.05mm -0.02mm       // asymmetric: upper then lower
9.98mm..10.05mm            // limits; nominal is the midpoint (see 5.2)
20mm H7                    // ISO 286 hole designation
20mm g6                    // ISO 286 shaft designation
```

- Deviations and limits MUST have the same dimension as the nominal;
  their unit may differ (`1in +- 0.1mm` is legal).
- ISO 286 designations resolve through the built-in fit tables
  (`std.fits.iso286`): uppercase letters are holes, lowercase are shafts.
  A size outside a table's range is F002.

### 5.1 Arithmetic

`+` and `-` on toleranced quantities use worst-case interval arithmetic
(deviations add). This is arithmetic, not a stack analysis: cadetta does
not claim completeness of a chain of contributors. `*` and `/` by an
untoleranced scalar scale the deviations. `*` and `/` of two toleranced
quantities are an error in 0.1.0 (Q005) because the result is not an
interval around a nominal in general.

### 5.2 Which number reaches the CAD tool

A CAD parameter holds one number. The emitted point is chosen by policy,
never by an emitter (adapters.md 4): `nominal` (default), `mid` (midpoint
of the limits), `mmc` or `lmc` (maximum or least material condition;
requires knowing hole vs shaft, which ISO designations and `fit` provide).
Policy is set in `cadetta.toml [emit] point` and overridable per
parameter with `@emit(mid)`.

### 5.3 Comparisons are three-valued

Comparing toleranced quantities (in `check` statements, language.md 6)
yields `holds` (true for every value in the intervals), `violates`
(false for every value) or `undecided` (true for some, false for others).
`undecided` is reported as C002 and never silently counted as `holds`.

## 6. Display

The kernel stores SI rationals; units are a display concern. Display
unit selection, in order: the parameter's `@display(unit)`, then
`[units] display` for the parameter's kind in `cadetta.toml`, then the
unit the value was authored in. The formatter preserves authored units.

## 7. Diagnostics owned by this document

Q001 unitless-quantity, Q002 dimension-mismatch, Q003 kind-mismatch,
Q004 inexact-emit, Q005 unsupported-toleranced-arithmetic, Q006
inexact-value, Q007 quantity-literal-split. Definitions in lints.md.

## Open questions

- OQ-1 (closed 2026-10-13): The original README showed `5.in`. `5.` is
  ambiguous with a decimal point, so the literal is `5in`; the README was
  rewritten to match.
- OQ-2: Statistical (RSS) tolerance arithmetic. Deferred; see lithos-2
  OQ-8 for why it should not be faked.
- OQ-3: Should packages be able to define new kinds of an existing
  dimension (e.g. Stress vs Pressure)? Recommendation: yes, via `kind
  Stress = Pressure` declarations in a later version.
