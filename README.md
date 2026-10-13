# Cadetta

This project serves as an obligation and source tracker and organizational tool
for CAD design. 

This is primarily for 3D CAD (namely SOLIDWORKS and other common tools), but there 
are plans to extend it to all kinds of CAD, such as electrical design (i.e. KiCAD)
and so forth.

---

## Parametric Design

Most CAD software have some sort of basic parametric features, but there's a problem
with simple, flat, labeled data.

- **Duplication**: Some geometry of an OTS part drives multiple dimensions across multiple
  parts; it is impossible to derive a single source of truth.
- **Naming**: "Diameter" applied to... every hole and cylindrical object in the design, so
  parameters are often prefixed with a name of a part. However, when humans produce these
  names, canonicalization is often not followed, meaning that you have to do work to figure
  out what a parameter means.
- **Magic Numbers**: Different kinds of fits requires different kinds of tolerances. Keeping
  track of all of this often leads to (1) manually entering the tolerance and the nominal 
  measurement, which is a hassle; and (2) when another engineer comes to look at a measurement
  it's hard to see whether a weirdly specific size is a design mistake or a toleranced fit.

---

## Quickstart

To start a project, run the command, `cadetta init`. A complete `cadetta.toml` will be produced
as well as a "hello, world" outline for your design.

```text
// declare namespace up here.

assembly Car {

}

model Axle {

}

ots Wheel {
   diameter = 5.in // alternatively, 5"
}

```



