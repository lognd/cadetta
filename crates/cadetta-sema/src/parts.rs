// frob:ticket 01M4ST7AKENKKS96TTXY13YMZQ
//! Part arrays, instance indexing and argument binding (language.md 3.1, 3.5).
//!
//! `part wheel[4]: Caster` has a Count `N` evaluated before any member
//! ([`array_count`]); `count(wheel)` is that Count, `wheel[i]` an instance
//! ([`PartInstances::get`]) and an unindexed `wheel.member` reads instance 0
//! ([`PartInstances::first`]). Arguments are bound to a component's declared
//! parameters with defaults applied by [`bind_args`]. The module is pure: the
//! evaluator supplies evaluated counts and stores the resulting instances.

use std::collections::BTreeSet;
use std::fmt;

use cadetta_syntax::ast::{Arg, Expr, ParamDecl, Type};
use cadetta_syntax::span::Span;
use cadetta_units::Dim;
use tracing::{debug, trace, warn};

use crate::eval::Quant;

/// Why a part array, index or argument list is invalid.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PartsError {
    /// The size `N` is not a dimensionless, tolerance-free whole number.
    NotACount(String),
    /// The size `N` is negative.
    NegativeCount(String),
    /// An index is outside `0..count`.
    IndexOutOfRange {
        /// The part name.
        part: String,
        /// The requested index.
        index: u64,
        /// The instance count.
        count: u32,
    },
    /// An unindexed read of an array with no instances.
    EmptyArray(String),
    /// More positional arguments than parameters.
    TooManyArgs {
        /// The component name.
        component: String,
        /// The parameter count.
        expected: usize,
    },
    /// A named argument that is not a parameter.
    UnknownArg(String),
    /// A parameter given twice (positionally and by name, or twice by name).
    DuplicateArg(String),
    /// A parameter with no argument and no default.
    MissingArg(String),
}

impl fmt::Display for PartsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotACount(s) => write!(f, "part count `{s}` is not a whole number"),
            Self::NegativeCount(s) => write!(f, "part count `{s}` is negative"),
            Self::IndexOutOfRange { part, index, count } => {
                write!(f, "`{part}[{index}]` is out of range: count is {count}")
            }
            Self::EmptyArray(p) => write!(f, "`{p}` has no instances to read a member from"),
            Self::TooManyArgs {
                component,
                expected,
            } => write!(f, "`{component}` takes {expected} argument(s)"),
            Self::UnknownArg(n) => write!(f, "no parameter named `{n}`"),
            Self::DuplicateArg(n) => write!(f, "parameter `{n}` is given more than once"),
            Self::MissingArg(n) => write!(f, "parameter `{n}` has no argument and no default"),
        }
    }
}

impl std::error::Error for PartsError {}

/// The Count of an array size `N`: a dimensionless, exact, non-negative whole number.
pub fn array_count(q: &Quant) -> Result<u32, PartsError> {
    let text = q.nominal.rational().to_string();
    if q.dim() != &Dim::default() || !q.form.is_exact() || q.nominal.pi_exp() != 0 {
        warn!(%text, "array size is not an exact dimensionless number");
        return Err(PartsError::NotACount(text));
    }
    let whole = q.nominal.rational().to_string();
    if whole.starts_with('-') {
        return Err(PartsError::NegativeCount(text));
    }
    let n = whole
        .parse::<u32>()
        .map_err(|_| PartsError::NotACount(text))?;
    debug!(count = n, "array size evaluated");
    Ok(n)
}

/// The instances of one `part name[N]` slot (a plain `part` has one).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PartInstances<T> {
    name: String,
    instances: Vec<T>,
}

impl<T> PartInstances<T> {
    /// The instances of part `name`, in index order.
    pub fn new(name: impl Into<String>, instances: Vec<T>) -> Self {
        Self {
            name: name.into(),
            instances,
        }
    }

    /// `count(name)`.
    pub fn count(&self) -> u32 {
        u32::try_from(self.instances.len()).unwrap_or(u32::MAX)
    }

    /// `name[i]`: instance `i`, or the out-of-range error.
    pub fn get(&self, i: u64) -> Result<&T, PartsError> {
        trace!(part = %self.name, index = i, "instance lookup");
        usize::try_from(i)
            .ok()
            .and_then(|i| self.instances.get(i))
            .ok_or_else(|| PartsError::IndexOutOfRange {
                part: self.name.clone(),
                index: i,
                count: self.count(),
            })
    }

    /// Unindexed `name.member` reads instance 0, as `name[0].member` does.
    pub fn first(&self) -> Result<&T, PartsError> {
        self.instances
            .first()
            .ok_or_else(|| PartsError::EmptyArray(self.name.clone()))
    }

    /// Every instance in index order, for `for x in name` and roll-ups.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.instances.iter()
    }
}

/// Where a parameter's value comes from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ArgSource<'a> {
    /// An argument written at the `part` site.
    Given(&'a Expr, Span),
    /// The parameter's declared default.
    Default(&'a Expr),
}

/// One parameter bound to its value expression.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BoundArg<'a> {
    /// The parameter name.
    pub name: &'a str,
    /// The parameter's declared type, so an enum argument may be a bare variant.
    pub ty: &'a Type,
    /// The value expression.
    pub source: ArgSource<'a>,
}

/// Binds `args` to `params` positionally then by name, applying defaults.
pub fn bind_args<'a>(
    component: &str,
    params: &'a [ParamDecl],
    args: &'a [Arg],
) -> Result<Vec<BoundArg<'a>>, PartsError> {
    let mut slots: Vec<Option<(&'a Expr, Span)>> = vec![None; params.len()];
    let mut positional = 0usize;
    let mut named = BTreeSet::new();
    for a in args {
        let idx = match &a.name {
            None => {
                let i = positional;
                positional += 1;
                if i >= params.len() {
                    return Err(PartsError::TooManyArgs {
                        component: component.to_owned(),
                        expected: params.len(),
                    });
                }
                i
            }
            Some(n) => {
                let Some(i) = params.iter().position(|p| p.name.name == n.name) else {
                    return Err(PartsError::UnknownArg(n.name.clone()));
                };
                if !named.insert(i) {
                    return Err(PartsError::DuplicateArg(n.name.clone()));
                }
                i
            }
        };
        if slots[idx].is_some() {
            return Err(PartsError::DuplicateArg(params[idx].name.name.clone()));
        }
        slots[idx] = Some((&a.value, a.span));
    }
    let mut out = Vec::with_capacity(params.len());
    for (p, slot) in params.iter().zip(slots) {
        let source = match (slot, &p.default) {
            (Some((e, s)), _) => ArgSource::Given(e, s),
            (None, Some(d)) => {
                debug!(component, param = %p.name.name, "default argument applied");
                ArgSource::Default(d)
            }
            (None, None) => return Err(PartsError::MissingArg(p.name.name.clone())),
        };
        out.push(BoundArg {
            name: &p.name.name,
            ty: &p.ty,
            source,
        });
    }
    Ok(out)
}

/// The full variant name of `bare` among an enum's `variants`, if it names one.
pub fn bare_variant<'a>(variants: &'a [String], bare: &str) -> Option<&'a str> {
    variants.iter().map(String::as_str).find(|v| *v == bare)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::ast::{ItemKind, Member, MemberKind};
    use cadetta_syntax::parser::parse;
    use cadetta_units::{Magnitude, Rational};

    fn count_of(n: i64) -> Quant {
        Quant::constant(Dim::default(), Magnitude::new(Rational::from_int(n), 0))
    }

    // frob:tests crates/cadetta-sema/src/parts.rs::array_count kind=unit
    #[test]
    fn count_of_four_is_a_count_and_bad_sizes_are_rejected() {
        assert_eq!(array_count(&count_of(4)), Ok(4));
        assert_eq!(array_count(&count_of(0)), Ok(0));
        assert!(matches!(
            array_count(&count_of(-1)),
            Err(PartsError::NegativeCount(_))
        ));
        let half = Quant::constant(
            Dim::default(),
            Magnitude::new(Rational::new(1, 2).unwrap(), 0),
        );
        assert!(matches!(array_count(&half), Err(PartsError::NotACount(_))));
    }

    // frob:tests crates/cadetta-sema/src/parts.rs::PartInstances.get kind=unit
    #[test]
    fn indexed_read_is_an_instance_and_out_of_range_errors() {
        let w = PartInstances::new("wheel", vec![10, 11, 12, 13]);
        assert_eq!(w.count(), 4);
        assert_eq!(w.get(2), Ok(&12));
        assert!(matches!(
            w.get(4),
            Err(PartsError::IndexOutOfRange { count: 4, .. })
        ));
    }

    // frob:tests crates/cadetta-sema/src/parts.rs::PartInstances.first kind=unit
    #[test]
    fn unindexed_read_equals_instance_zero() {
        let w = PartInstances::new("wheel", vec![10, 11]);
        assert_eq!(w.first(), w.get(0));
        let e: PartInstances<i32> = PartInstances::new("none", vec![]);
        assert_eq!(e.first(), Err(PartsError::EmptyArray("none".into())));
    }

    fn params(src: &str) -> Vec<ParamDecl> {
        let p = parse(src);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        for item in p.file.items() {
            if let ItemKind::Component(c) = &item.kind {
                return c.params.clone();
            }
        }
        panic!("no component");
    }

    fn call_args(src: &str) -> Vec<Arg> {
        let p = parse(&format!("assembly A {{ part s: Iso4762{src} }}"));
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        for item in p.file.items() {
            if let ItemKind::Component(c) = &item.kind {
                for Member { kind, .. } in &c.members {
                    if let MemberKind::Part(part) = kind {
                        return part.args.clone().unwrap_or_default();
                    }
                }
            }
        }
        panic!("no part");
    }

    const DECL: &str = "ots Iso4762(thread: MetricThread, length: Length, grade: Grade = G12) {}";

    // frob:tests crates/cadetta-sema/src/parts.rs::bind_args kind=unit
    #[test]
    fn defaults_apply_and_bare_enum_variants_bind_positionally() {
        let ps = params(DECL);
        let args = call_args("(M5, 25mm)");
        let bound = bind_args("Iso4762", &ps, &args).unwrap();
        assert_eq!(bound.len(), 3);
        assert!(matches!(bound[0].source, ArgSource::Given(..)));
        assert!(matches!(bound[2].source, ArgSource::Default(_)));
        let variants = vec!["M4".to_owned(), "M5".to_owned()];
        assert_eq!(bare_variant(&variants, "M5"), Some("M5"));
        assert_eq!(bare_variant(&variants, "M6"), None);
    }

    // frob:tests crates/cadetta-sema/src/parts.rs::bind_args kind=unit
    #[test]
    fn named_override_and_argument_errors() {
        let ps = params(DECL);
        let named = call_args("(thread = M5, length = 25mm, grade = G8)");
        let bound = bind_args("Iso4762", &ps, &named).unwrap();
        assert!(matches!(bound[2].source, ArgSource::Given(..)));
        let missing = call_args("(M5)");
        assert_eq!(
            bind_args("Iso4762", &ps, &missing),
            Err(PartsError::MissingArg("length".into()))
        );
        let unknown = call_args("(M5, 25mm, bogus = 1)");
        assert_eq!(
            bind_args("Iso4762", &ps, &unknown),
            Err(PartsError::UnknownArg("bogus".into()))
        );
        let dup = call_args("(M5, 25mm, thread = M4)");
        assert!(matches!(
            bind_args("Iso4762", &ps, &dup),
            Err(PartsError::DuplicateArg(_))
        ));
    }
}
