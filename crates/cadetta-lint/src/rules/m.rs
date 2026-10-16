//! Lints M001-M003: magic numbers.
//!
//! M001 flags an authored literal written with more decimals than the
//! configured precision for its kind and nothing to explain it. M002 flags a
//! bare dimensionless factor that is not a small integer, simple fraction or
//! power of ten. M003 flags a fit side with no tolerance. A tolerance or a fit
//! explains a literal; a table lookup or an `allow` never reaches these rules
//! (a lookup is not a literal, and the engine applies waivers). Provenance and
//! doc comments do not explain a value in a `model`; a citation does in an `ots`.
//!
//! Specification: `docs/spec/lints.md` (M), `docs/spec/config-cli.md` (`[lint] precision`).

// frob:ticket 01M4SPWQZF9T7HA75ESE6D2C96

use std::collections::BTreeSet;

use cadetta_ir::model::{
    Component, ComponentClass, Member, MemberKind, Nominal, Project, SpanRef, Value,
};
use cadetta_units::{Dim, Dimension};
use tracing::{debug, trace};

use crate::diag::{Diagnostic, Location};
use crate::engine::LintConfig;

/// Rule code: a literal has more decimals than the configured precision.
pub const M001: &str = "M001";
/// Rule code: a bare factor that is not a small integer, simple fraction or power of ten.
pub const M002: &str = "M002";
/// Rule code: a fit side has no tolerance.
pub const M003: &str = "M003";

/// Largest absolute integer, and largest numerator or denominator, treated as unremarkable.
const SMALL: i64 = 10;

/// The `[lint] precision` setting: maximum decimals per kind of quantity.
#[derive(Clone, Debug)]
pub struct MConfig {
    /// Maximum authored decimals per dimension; a kind not listed is never reported by M001.
    pub precision: Vec<(Dim, u32)>,
}

impl Default for MConfig {
    /// The documented example: Length = 2, Angle = 1.
    fn default() -> Self {
        let length = Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0);
        let angle = Dim::new(Dimension::dimensionless(), 1);
        Self {
            precision: vec![(length, 2), (angle, 1)],
        }
    }
}

impl MConfig {
    fn limit(&self, dim: &Dim) -> Option<u32> {
        self.precision
            .iter()
            .find(|(d, _)| d == dim)
            .map(|(_, n)| *n)
    }
}

/// Decimals in the authored numeric token, ignoring trailing zeros (`12.50mm` has 1).
pub fn authored_decimals(authored: &str) -> u32 {
    let s = authored.trim_start_matches(['+', '-']);
    let token: String = s
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '_')
        .collect();
    let Some((_, frac)) = token.split_once('.') else {
        return 0;
    };
    u32::try_from(frac.trim_end_matches('0').len()).unwrap_or(u32::MAX)
}

fn all_members(project: &Project) -> impl Iterator<Item = (&Component, &Member)> {
    project
        .components
        .iter()
        .flat_map(|c| c.members.iter().map(move |m| (c, m)))
}

/// True for a parameter written as a literal: no expression and nothing read.
fn is_literal(m: &Member) -> bool {
    m.kind == MemberKind::Param && crate::rules::d::is_literal(m)
}

/// Canonical text of every path used as a side of a fit.
fn fit_sides(project: &Project) -> BTreeSet<String> {
    project
        .components
        .iter()
        .flat_map(|c| c.fits.iter())
        .flat_map(|f| [f.hole.to_string(), f.shaft.to_string()])
        .collect()
}

fn is_cited(c: &Component, m: &Member) -> bool {
    m.citation.is_some() || c.citation.is_some()
}

/// M001: a literal with more decimals than the precision for its kind and no explanation.
pub fn check_m001(
    project: &Project,
    cfg: &LintConfig,
    mcfg: &MConfig,
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let sev = cfg.severity(M001).expect("M001 is in the catalog");
    let sides = fit_sides(project);
    let mut out = Vec::new();
    for (c, m) in all_members(project) {
        let Some(v) = m.value.as_ref().filter(|_| is_literal(m)) else {
            continue;
        };
        let Some(limit) = mcfg.limit(&v.dim) else {
            trace!(path = %m.path, "M001: no precision for this kind");
            continue;
        };
        let decimals = authored_decimals(&v.authored);
        if decimals <= limit {
            continue;
        }
        if v.tol.is_some() || sides.contains(&m.path.to_string()) {
            trace!(path = %m.path, "M001: explained by a tolerance or fit");
            continue;
        }
        if c.class == ComponentClass::Ots && is_cited(c, m) {
            trace!(path = %m.path, "M001: cited ots value");
            continue;
        }
        debug!(path = %m.path, decimals, limit, "M001: unexplained precision");
        let mut d = Diagnostic::new(
            M001,
            sev,
            format!(
                "literal `{}` has {decimals} decimals (precision {limit}) and nothing explains it",
                v.authored
            ),
        );
        d.path = Some(m.path.to_string());
        d.location = m.span.as_ref().map(&locate);
        out.push(d);
    }
    out
}

/// True when `(num, den)` is a power of ten or its reciprocal: `10^k` or `1/10^k`.
fn is_power_of_ten(num: &str, den: &str) -> bool {
    let pow = |s: &str| s.starts_with('1') && s[1..].chars().all(|c| c == '0');
    (den == "1" && pow(num)) || (num == "1" && pow(den))
}

/// True for a factor nobody questions: a small integer, a simple fraction or a power of ten.
pub fn is_plain_factor(value: &Value) -> bool {
    let Nominal::Exact(e) = &value.nominal else {
        return false;
    };
    if e.0.pi_exp() != 0 {
        return false;
    }
    let r = e.0.rational();
    let num = r.numer().to_string();
    let den = r.denom().to_string();
    let num_abs = num.trim_start_matches('-');
    if is_power_of_ten(num_abs, &den) {
        return true;
    }
    match (num_abs.parse::<i64>(), den.parse::<i64>()) {
        (Ok(n), Ok(d)) => n <= SMALL && d <= SMALL,
        _ => false,
    }
}

/// M002: a bare dimensionless literal that is not plain; info severity.
pub fn check_m002(
    project: &Project,
    cfg: &LintConfig,
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let sev = cfg.severity(M002).expect("M002 is in the catalog");
    let mut out = Vec::new();
    for (c, m) in all_members(project) {
        let Some(v) = m.value.as_ref().filter(|_| is_literal(m)) else {
            continue;
        };
        if v.dim != Dim::default() || is_plain_factor(v) || is_cited(c, m) {
            continue;
        }
        if !matches!(v.nominal, Nominal::Exact(_)) {
            continue;
        }
        debug!(path = %m.path, "M002: magic factor");
        let mut d = Diagnostic::new(
            M002,
            sev,
            format!(
                "bare factor `{}` is unexplained; name it or cite it",
                v.authored
            ),
        );
        d.path = Some(m.path.to_string());
        d.location = m.span.as_ref().map(&locate);
        out.push(d);
    }
    out
}

/// M003: a parameter used as a side of a fit that carries no tolerance.
pub fn check_m003(
    project: &Project,
    cfg: &LintConfig,
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let sev = cfg.severity(M003).expect("M003 is in the catalog");
    let sides = fit_sides(project);
    let mut out = Vec::new();
    for (_, m) in all_members(project) {
        let path = m.path.to_string();
        if !sides.contains(&path) || m.value.as_ref().is_some_and(|v| v.tol.is_some()) {
            continue;
        }
        debug!(path = %path, "M003: fit side without tolerance");
        let mut d = Diagnostic::new(
            M003,
            sev,
            format!("`{path}` is a side of a fit but has no tolerance"),
        );
        d.path = Some(path);
        d.location = m.span.as_ref().map(&locate);
        out.push(d);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Severity;
    use cadetta_ir::model::{
        // frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
        Affine,
        AffineTerm,
        CiteRef,
        ComponentClass,
        Exact,
        Fit,
        Flavour,
        PackageId,
        Provenance,
    };
    use cadetta_ir::path::{Path, PathContext};
    use cadetta_units::{Magnitude, Rational};

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    fn length() -> Dim {
        Dim::new(Dimension::from_si([1, 0, 0, 0, 0, 0, 0]), 0)
    }

    fn exact(n: i64, d: i64) -> Exact {
        Exact(Magnitude::new(Rational::new(n, d).unwrap(), 0))
    }

    fn value(n: i64, d: i64, dim: Dim, authored: &str) -> Value {
        Value::new(Nominal::Exact(exact(n, d)), dim, authored)
    }

    fn member(name: &str, v: Value) -> Member {
        Member {
            path: p(name),
            kind: MemberKind::Param,
            analysis: false,
            private: false,
            value: Some(v),
            bound: None,
            expr: None,
            deps: vec![],
            role: None,
            citation: None,
            was: vec![],
            attrs: Default::default(),
            provenance: Provenance::Authored,
            doc: None,
            span: None,
            variants: None,
            digest: None,
            closure: None,
            text: None,
        }
    }

    fn cite() -> CiteRef {
        CiteRef::item("demo::iso286", None)
    }

    fn project(class: ComponentClass, members: Vec<Member>, fits: Vec<Fit>) -> Project {
        let mut proj = Project::new(
            Flavour::Design,
            PackageId {
                name: "demo".into(),
                version: "0.1.0".into(),
            },
        );
        proj.components.push(Component {
            path: p("demo::A.x"),
            class,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: Default::default(),
            members,
            parts: vec![],
            fits,
            datums: vec![],
            points: vec![],
            placements: vec![],
            digest: None,
            span: None,
        });
        proj
    }

    fn fit(hole: &str, shaft: &str) -> Fit {
        Fit {
            path: p("demo::A.seat"),
            hole: p(hole),
            shaft: p(shaft),
            clearance: value(0, 1, length(), "0mm"),
            fit_type: "clearance".into(),
            required: None,
        }
    }

    fn loc(_: &SpanRef) -> Location {
        unreachable!("no spans in these fixtures")
    }

    fn cfg() -> LintConfig {
        LintConfig::new()
    }

    /// `12.012mm` as a length literal (thousandths of a millimetre, in metres).
    fn precise() -> Value {
        value(12_012, 1_000_000, length(), "12.012mm")
    }

    // frob:tests m001_fires_on_unexplained_model_literal
    #[test]
    fn m001_fires_on_unexplained_model_literal() {
        let proj = project(
            ComponentClass::Model,
            vec![member("demo::A.w", precise())],
            vec![],
        );
        let out = check_m001(&proj, &cfg(), &MConfig::default(), loc);
        assert_eq!(out.len(), 1);
        assert_eq!(
            (out[0].code.as_str(), out[0].severity),
            (M001, Severity::Warning)
        );
        // a citation does not explain a model literal
        let mut m = member("demo::A.w", precise());
        m.citation = Some(cite());
        m.doc = Some("why".into());
        let proj = project(ComponentClass::Model, vec![m], vec![]);
        assert_eq!(check_m001(&proj, &cfg(), &MConfig::default(), loc).len(), 1);
    }

    // frob:tests m001_silent_when_cited_ots_toleranced_fit_or_coarse
    #[test]
    fn m001_silent_when_cited_ots_toleranced_fit_or_coarse() {
        let mut m = member("demo::A.w", precise());
        m.citation = Some(cite());
        let ots = project(ComponentClass::Ots, vec![m], vec![]);
        assert!(check_m001(&ots, &cfg(), &MConfig::default(), loc).is_empty());
        let uncited = project(
            ComponentClass::Ots,
            vec![member("demo::A.w", precise())],
            vec![],
        );
        assert_eq!(
            check_m001(&uncited, &cfg(), &MConfig::default(), loc).len(),
            1
        );

        let mut v = precise();
        v.tol = Some(Affine {
            terms: vec![AffineTerm {
                var: p("demo::A.w"),
                lo: exact(-1, 1000),
                hi: exact(1, 1000),
            }],
        });
        let tol = project(ComponentClass::Model, vec![member("demo::A.w", v)], vec![]);
        assert!(check_m001(&tol, &cfg(), &MConfig::default(), loc).is_empty());

        let fitted = project(
            ComponentClass::Model,
            vec![member("demo::A.w", precise())],
            vec![fit("demo::A.w", "demo::A.s")],
        );
        assert!(check_m001(&fitted, &cfg(), &MConfig::default(), loc).is_empty());

        let coarse = value(125, 100_000, length(), "12.50mm");
        let proj = project(
            ComponentClass::Model,
            vec![member("demo::A.w", coarse)],
            vec![],
        );
        assert!(check_m001(&proj, &cfg(), &MConfig::default(), loc).is_empty());
    }

    // frob:tests authored_decimals_ignores_trailing_zeros
    #[test]
    fn authored_decimals_ignores_trailing_zeros() {
        assert_eq!(authored_decimals("12.012mm"), 3);
        assert_eq!(authored_decimals("12.50mm"), 1);
        assert_eq!(authored_decimals("100mm"), 0);
        assert_eq!(authored_decimals("-0.25"), 2);
    }

    // frob:tests m002_fires_on_odd_factor_only
    #[test]
    fn m002_fires_on_odd_factor_only() {
        let odd = member("demo::A.k", value(1237, 1000, Dim::default(), "1.237"));
        let proj = project(ComponentClass::Model, vec![odd], vec![]);
        let out = check_m002(&proj, &cfg(), loc);
        assert_eq!(out.len(), 1);
        assert_eq!(
            (out[0].code.as_str(), out[0].severity),
            (M002, Severity::Info)
        );

        for (n, d, text) in [
            (2, 1, "2"),
            (-3, 1, "-3"),
            (1, 2, "0.5"),
            (3, 4, "0.75"),
            (1000, 1, "1000"),
            (1, 100, "0.01"),
        ] {
            let ok = member("demo::A.k", value(n, d, Dim::default(), text));
            let proj = project(ComponentClass::Model, vec![ok], vec![]);
            assert!(check_m002(&proj, &cfg(), loc).is_empty(), "{text} is plain");
        }
        // a dimensioned literal is not a factor
        let len = project(
            ComponentClass::Model,
            vec![member("demo::A.w", precise())],
            vec![],
        );
        assert!(check_m002(&len, &cfg(), loc).is_empty());
    }

    // frob:tests m003_fires_on_untoleranced_fit_side
    #[test]
    fn m003_fires_on_untoleranced_fit_side() {
        let w = value(12, 1000, length(), "12mm");
        let proj = project(
            ComponentClass::Model,
            vec![member("demo::A.h", w.clone()), member("demo::A.other", w)],
            vec![fit("demo::A.h", "demo::A.s")],
        );
        let out = check_m003(&proj, &cfg(), loc);
        assert_eq!(out.len(), 1);
        assert_eq!(
            (out[0].code.as_str(), out[0].severity),
            (M003, Severity::Warning)
        );
        assert_eq!(out[0].path.as_deref(), Some("demo::A.h"));
    }

    // frob:tests m003_silent_when_side_has_tolerance
    #[test]
    fn m003_silent_when_side_has_tolerance() {
        let mut w = value(12, 1000, length(), "12mm");
        w.tol = Some(Affine {
            terms: vec![AffineTerm {
                var: p("demo::A.h"),
                lo: exact(0, 1),
                hi: exact(1, 1000),
            }],
        });
        let proj = project(
            ComponentClass::Model,
            vec![member("demo::A.h", w)],
            vec![fit("demo::A.h", "demo::A.s")],
        );
        assert!(check_m003(&proj, &cfg(), loc).is_empty());
    }
}
