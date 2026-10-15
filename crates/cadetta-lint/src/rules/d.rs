//! Lints D001-D003: duplication and single source of truth.
//!
//! D001 groups authored literals that restate another parameter's value, D002
//! flags a literal the design already offers by name (a fit member accessor, a
//! table cell, a solver output), and D003 finds reference cycles between
//! members. D003 is a fixed error and cannot be waived.
//!
//! Specification: `docs/spec/lints.md` (D), `docs/spec/language.md` 4.

// frob:ticket 01M4SPWQWV4J8DF3B1RF4PFYCT

use std::collections::{BTreeMap, BTreeSet};

use cadetta_ir::model::{
    Component, ComponentClass, Member, MemberKind, Nominal, Project, Provenance, SpanRef, Value,
};
use cadetta_ir::path::Path;
use cadetta_units::{Dim, Magnitude, Sum};
use tracing::{debug, trace};

use crate::diag::{Diagnostic, Location, Note, Severity};
use crate::engine::LintConfig;

/// Rule code: a literal restates another parameter's value.
pub const D001: &str = "D001";
/// Rule code: a literal equals a value the design offers by name.
pub const D002: &str = "D002";
/// Rule code: members depend on each other cyclically (fixed error).
pub const D003: &str = "D003";

/// The default `[lint] d001_min_digits`.
pub const DEFAULT_MIN_DIGITS: u32 = 3;

/// A value of a given kind, as compared by D001 and D002.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key {
    dim: String,
    mag: String,
}

impl Key {
    /// The comparison key of an exact magnitude of dimension `dim`.
    pub fn new(dim: &Dim, mag: &Magnitude) -> Self {
        Self {
            dim: format!("{dim:?}"),
            mag: format!("{mag:?}"),
        }
    }

    /// True when both keys are of the same dimension (T001 compares like with like).
    // frob:ticket 01M4TRKDAC6FR44KEW64H7PTZX
    pub fn same_dim(&self, other: &Key) -> bool {
        self.dim == other.dim
    }
}

/// A value the design offers by name, such as a table cell reachable from a component.
#[derive(Clone, Debug)]
pub struct NamedValue {
    /// The name shown in the diagnostic, for example `Bearing.table[608].bore`.
    pub name: String,
    /// The comparison key of the value.
    pub key: Key,
}

/// The inputs D001 and D002 need beyond the IR: `[lint]` settings and external references.
#[derive(Clone, Debug)]
pub struct DConfig {
    /// `[lint] d001_min_digits`: significant digits needed for the configured severity.
    pub min_digits: u32,
    /// `[lint] common_values`, parsed by sema into comparison keys.
    pub common_values: Vec<Key>,
    /// Table cells reachable from components, supplied by the caller (the IR carries no cells).
    pub table_cells: Vec<NamedValue>,
}

impl Default for DConfig {
    fn default() -> Self {
        Self {
            min_digits: DEFAULT_MIN_DIGITS,
            common_values: Vec::new(),
            table_cells: Vec::new(),
        }
    }
}

/// Significant digits in the authored form, first to last non-zero digit (`100mm` is 1).
pub fn significant_digits(authored: &str) -> u32 {
    let s = authored.trim_start_matches(['+', '-']);
    let token: String = s
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '_')
        .filter(char::is_ascii_digit)
        .collect();
    let n = token.trim_matches('0').len();
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn exact(v: &Value) -> Option<&Magnitude> {
    match &v.nominal {
        Nominal::Exact(e) => Some(&e.0),
        Nominal::Inexact(_) => None,
    }
}

fn plus(a: &Magnitude, b: &Magnitude) -> Option<Magnitude> {
    match a.add(b) {
        Sum::Exact(m) => Some(m),
        Sum::Inexact(_) => None,
    }
}

/// Nominal, min, max and mid of a value, as `(accessor, key)`; tolerance-free values give nominal only.
fn accessors(v: &Value) -> Vec<(&'static str, Key)> {
    let Some(nom) = exact(v) else {
        return Vec::new();
    };
    let mut out = vec![("nominal", Key::new(&v.dim, nom))];
    let Some(tol) = &v.tol else {
        return out;
    };
    let (mut lo, mut hi) = (nom.clone(), nom.clone());
    for t in &tol.terms {
        let (Some(l), Some(h)) = (plus(&lo, &t.lo.0), plus(&hi, &t.hi.0)) else {
            return out;
        };
        lo = l;
        hi = h;
    }
    out.push(("min", Key::new(&v.dim, &lo)));
    out.push(("max", Key::new(&v.dim, &hi)));
    if let Some(sum) = plus(&lo, &hi)
        && let Ok(half) = cadetta_units::Rational::new(1, 2)
    {
        out.push(("mid", Key::new(&v.dim, &sum.mul(&Magnitude::new(half, 0)))));
    }
    out
}

fn is_imported(m: &Member) -> bool {
    matches!(m.provenance, Provenance::Imported { .. })
}

/// A parameter with a stored, exact, non-derived value.
fn stated(m: &Member) -> Option<(Key, &Value)> {
    if m.kind != MemberKind::Param || m.provenance == Provenance::Derived {
        return None;
    }
    let v = m.value.as_ref()?;
    Some((Key::new(&v.dim, exact(v)?), v))
}

/// True for a literal as written: nothing read, and either no expression text or text that
/// is just the authored token (lowering records the source text of a literal as `expr`).
/// A table lookup such as `Iso273[6mm].pitch` reads a cell, so it is not a literal even
/// though its evaluated value records the same text and it carries no member deps.
pub fn is_literal(m: &Member) -> bool {
    m.provenance == Provenance::Authored
        && m.deps.is_empty()
        && m.value.as_ref().is_none_or(|v| {
            v.authored
                .starts_with(|c: char| c.is_ascii_digit() || "+-.".contains(c))
        })
        && match (&m.expr, &m.value) {
            (None, _) => true,
            (Some(e), Some(v)) => *e == v.authored,
            (Some(_), None) => false,
        }
}

fn all_members(project: &Project) -> impl Iterator<Item = (&Component, &Member)> {
    project
        .components
        .iter()
        .flat_map(|c| c.members.iter().map(move |m| (c, m)))
}

fn find_member<'a>(project: &'a Project, path: &Path) -> Option<&'a Member> {
    let want = path.to_string();
    all_members(project)
        .map(|(_, m)| m)
        .find(|m| m.path.to_string() == want)
}

/// D001: one diagnostic per group of equal values with a literal in it, listing every location.
pub fn check_d001(
    project: &Project,
    cfg: &LintConfig,
    dcfg: &DConfig,
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let sev = cfg.severity(D001).expect("D001 is in the catalog");
    let mut groups: BTreeMap<Key, Vec<(&Component, &Member, &Value)>> = BTreeMap::new();
    for (c, m) in all_members(project) {
        if let Some((k, v)) = stated(m) {
            groups.entry(k).or_default().push((c, m, v));
        }
    }
    let mut out = Vec::new();
    for (key, group) in groups {
        let has_subject = group
            .iter()
            .any(|(c, m, _)| c.class != ComponentClass::Ots && is_literal(m));
        if group.len() < 2 || !has_subject {
            trace!(?key, "D001: no restated literal");
            continue;
        }
        let digits = group
            .iter()
            .map(|(_, _, v)| significant_digits(&v.authored))
            .max()
            .unwrap_or(0);
        let imported = group.iter().any(|(_, m, _)| is_imported(m));
        if dcfg.common_values.contains(&key) && !imported {
            debug!(?key, "D001: common value, not reported");
            continue;
        }
        let severity = if digits >= dcfg.min_digits {
            sev
        } else {
            Severity::Info
        };
        let (_, first, first_v) = group[0];
        debug!(path = %first.path, digits, n = group.len(), "D001: restated value");
        let mut d = Diagnostic::new(
            D001,
            severity,
            format!(
                "value `{}` is stated {} times; state it once and derive the rest",
                first_v.authored,
                group.len()
            ),
        );
        d.path = Some(first.path.to_string());
        d.location = first.span.as_ref().map(&locate);
        d.notes = group
            .iter()
            .map(|(_, m, v)| Note {
                message: format!("`{}` = {}", m.path, v.authored),
                location: m.span.as_ref().map(&locate),
            })
            .collect();
        out.push(d);
    }
    out
}

/// D002: a literal equal to a fit member accessor, a table cell or a solver output.
pub fn check_d002(
    project: &Project,
    cfg: &LintConfig,
    dcfg: &DConfig,
    locate: impl Fn(&SpanRef) -> Location,
) -> Vec<Diagnostic> {
    let sev = cfg.severity(D002).expect("D002 is in the catalog");
    // key -> (source name, source member path text if any)
    let mut offered: BTreeMap<Key, Vec<(String, Option<String>)>> = BTreeMap::new();
    let mut add = |name: String, v: &Value, own: Option<String>| {
        for (acc, k) in accessors(v) {
            offered
                .entry(k)
                .or_default()
                .push((format!("{name}.{acc}"), own.clone()));
        }
    };
    for c in &project.components {
        for f in &c.fits {
            for (role, p) in [("hole", &f.hole), ("shaft", &f.shaft)] {
                if let Some(v) = find_member(project, p).and_then(|m| m.value.as_ref()) {
                    add(format!("{}.{role}", f.path), v, Some(p.to_string()));
                }
            }
            add(format!("{}.clearance", f.path), &f.clearance, None);
        }
    }
    for s in &project.solvers {
        for out_path in s.outputs.values() {
            let Ok(p) = out_path.parse::<Path>() else {
                continue;
            };
            if let Some(v) = find_member(project, &p).and_then(|m| m.value.as_ref()) {
                add(out_path.clone(), v, Some(p.to_string()));
            }
        }
    }
    for cell in &dcfg.table_cells {
        offered
            .entry(cell.key.clone())
            .or_default()
            .push((cell.name.clone(), None));
    }
    let mut out = Vec::new();
    for (c, m) in all_members(project) {
        if c.class == ComponentClass::Ots || !is_literal(m) {
            continue;
        }
        let Some((key, v)) = stated(m) else { continue };
        let own = m.path.to_string();
        let Some((name, _)) = offered.get(&key).and_then(|srcs| {
            srcs.iter()
                .find(|(_, o)| o.as_deref() != Some(own.as_str()))
        }) else {
            continue;
        };
        debug!(path = %m.path, source = %name, "D002: derivable literal");
        let mut d = Diagnostic::new(
            D002,
            sev,
            format!(
                "literal `{}` equals `{name}`; reference it instead of restating it",
                v.authored
            ),
        );
        d.path = Some(own);
        d.location = m.span.as_ref().map(&locate);
        out.push(d);
    }
    out
}

/// Identity of a path ignoring instance steps.
fn key_of(p: &Path) -> String {
    Path {
        steps: Vec::new(),
        ..p.clone()
    }
    .to_string()
}

/// D003: one error per cycle among members' `deps`, naming every member of the cycle.
pub fn check_d003(project: &Project, locate: impl Fn(&SpanRef) -> Location) -> Vec<Diagnostic> {
    let members: Vec<&Member> = all_members(project).map(|(_, m)| m).collect();
    let index: BTreeMap<String, usize> = members
        .iter()
        .enumerate()
        .map(|(i, m)| (key_of(&m.path), i))
        .collect();
    let edges: Vec<Vec<usize>> = members
        .iter()
        .map(|m| {
            let mut e: Vec<usize> = m
                .deps
                .iter()
                .filter_map(|d| {
                    let k = key_of(d);
                    index.get(&k).copied().or_else(|| {
                        // a read of a sub-member (`a.b.c`) depends on the nearest member `a.b`
                        k.rfind('.').and_then(|i| index.get(&k[..i]).copied())
                    })
                })
                .collect();
            e.sort_unstable();
            e.dedup();
            e
        })
        .collect();
    let mut out = Vec::new();
    for scc in tarjan(&edges) {
        let self_loop = scc.len() == 1 && edges[scc[0]].contains(&scc[0]);
        if scc.len() < 2 && !self_loop {
            continue;
        }
        let names: BTreeSet<String> = scc.iter().map(|&i| members[i].path.to_string()).collect();
        let first = scc
            .iter()
            .map(|&i| members[i])
            .min_by_key(|m| m.path.to_string())
            .expect("scc is nonempty");
        debug!(path = %first.path, size = scc.len(), "D003: reference cycle");
        let mut d = Diagnostic::new(
            D003,
            Severity::Error,
            format!(
                "reference cycle: {}",
                names.iter().cloned().collect::<Vec<_>>().join(" -> ")
            ),
        );
        d.path = Some(first.path.to_string());
        d.location = first.span.as_ref().map(&locate);
        out.push(d);
    }
    out
}

/// Strongly connected components of a graph given as adjacency lists (iterative Tarjan).
fn tarjan(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = edges.len();
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut on = vec![false; n];
    let mut stack = Vec::new();
    let mut next = 0usize;
    let mut sccs = Vec::new();
    for root in 0..n {
        if index[root] != usize::MAX {
            continue;
        }
        let mut work = vec![(root, 0usize)];
        while let Some(&(v, i)) = work.last() {
            if i == 0 {
                index[v] = next;
                low[v] = next;
                next += 1;
                stack.push(v);
                on[v] = true;
            }
            if let Some(&w) = edges[v].get(i) {
                work.last_mut().expect("nonempty").1 += 1;
                if index[w] == usize::MAX {
                    work.push((w, 0));
                } else if on[w] {
                    low[v] = low[v].min(index[w]);
                }
            } else {
                work.pop();
                if let Some(&(parent, _)) = work.last() {
                    low[parent] = low[parent].min(low[v]);
                }
                if low[v] == index[v] {
                    let mut scc = Vec::new();
                    while let Some(w) = stack.pop() {
                        on[w] = false;
                        scc.push(w);
                        if w == v {
                            break;
                        }
                    }
                    sccs.push(scc);
                }
            }
        }
    }
    sccs
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_ir::model::{Exact, Fit, Flavour, PackageId, Solver};
    use cadetta_ir::path::PathContext;
    use cadetta_units::Rational;

    fn p(s: &str) -> Path {
        Path::parse(s, PathContext::General).unwrap()
    }

    /// Millimetres as metres: `thousandths` / 1_000_000.
    fn mm(thousandths: i64) -> Magnitude {
        Magnitude::new(Rational::new(thousandths, 1_000_000).unwrap(), 0)
    }

    fn val(thousandths: i64, authored: &str) -> Value {
        Value::new(
            Nominal::Exact(Exact(mm(thousandths))),
            Dim::default(),
            authored,
        )
    }

    fn param(path: &str, v: Value) -> Member {
        let mut m = Member::new(p(path), MemberKind::Param);
        m.value = Some(v);
        m
    }

    fn comp(path: &str, class: ComponentClass, members: Vec<Member>) -> Component {
        Component {
            path: p(path),
            class,
            params_sig: None,
            implements: vec![],
            citation: None,
            attrs: Default::default(),
            members,
            parts: vec![],
            fits: vec![],
            datums: vec![],
            points: vec![],
            placements: vec![],
            span: None,
            digest: None,
        }
    }

    fn project(components: Vec<Component>) -> Project {
        let mut proj = Project::new(
            Flavour::Design,
            PackageId {
                name: "demo".into(),
                version: "0.1.0".into(),
            },
        );
        proj.components = components;
        proj
    }

    fn two_models(a: Value, b: Value) -> Project {
        project(vec![
            comp(
                "demo::A.x",
                ComponentClass::Model,
                vec![param("demo::A.x", a)],
            ),
            comp(
                "demo::B.y",
                ComponentClass::Model,
                vec![param("demo::B.y", b)],
            ),
        ])
    }

    fn d001(proj: &Project, dcfg: &DConfig) -> Vec<Diagnostic> {
        check_d001(proj, &LintConfig::new(), dcfg, |_| unreachable!())
    }

    #[test]
    fn digits_counted_first_to_last_nonzero() {
        assert_eq!(significant_digits("12.012mm"), 5);
        assert_eq!(significant_digits("6.35mm"), 3);
        assert_eq!(significant_digits("100mm"), 1);
        assert_eq!(significant_digits("0.050mm"), 1);
    }

    // frob:tests crates/cadetta-lint/src/rules/d.rs::check_d001
    #[test]
    fn d001_fires_once_listing_all_locations() {
        let proj = two_models(val(12_012, "12.012mm"), val(12_012, "12.012mm"));
        let d = d001(&proj, &DConfig::default());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "D001");
        assert_eq!(d[0].severity, Severity::Warning);
        assert_eq!(d[0].notes.len(), 2);
    }

    // frob:tests crates/cadetta-lint/src/rules/d.rs::check_d001
    #[test]
    fn d001_info_for_low_specificity_and_common_values() {
        let proj = two_models(val(100_000, "100mm"), val(100_000, "100mm"));
        let d = d001(&proj, &DConfig::default());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].severity, Severity::Info);

        let key = Key::new(&Dim::default(), &mm(100_000));
        let common = DConfig {
            common_values: vec![key],
            ..DConfig::default()
        };
        assert!(d001(&proj, &common).is_empty());

        let mut imported = proj.clone();
        imported.components[0].members[0].provenance = Provenance::Imported {
            adapter: "freecad".into(),
            file: "a.FCStd".into(),
            native_name: "x".into(),
        };
        assert_eq!(d001(&imported, &common).len(), 1);
    }

    // frob:tests crates/cadetta-lint/src/rules/d.rs::check_d001
    #[test]
    fn d001_quiet_between_two_ots_and_for_distinct_values() {
        let ots = project(vec![
            comp(
                "demo::A.x",
                ComponentClass::Ots,
                vec![param("demo::A.x", val(12_012, "12.012mm"))],
            ),
            comp(
                "demo::B.y",
                ComponentClass::Ots,
                vec![param("demo::B.y", val(12_012, "12.012mm"))],
            ),
        ]);
        assert!(d001(&ots, &DConfig::default()).is_empty());
        let distinct = two_models(val(12_012, "12.012mm"), val(12_013, "12.013mm"));
        assert!(d001(&distinct, &DConfig::default()).is_empty());
    }

    fn fit_project(literal: Value) -> Project {
        let mut seat = comp(
            "demo::Seat.fit",
            ComponentClass::Model,
            vec![
                param("demo::Seat.shaft", {
                    let mut v = val(12_010, "12.010mm");
                    v.tol = Some(cadetta_ir::model::Affine {
                        terms: vec![cadetta_ir::model::AffineTerm {
                            var: p("demo::Seat.shaft"),
                            lo: Exact(mm(-2)),
                            hi: Exact(mm(0)),
                        }],
                    });
                    v
                }),
                param("demo::Seat.hole", val(12_100, "12.100mm")),
            ],
        );
        seat.fits.push(Fit {
            path: p("demo::Seat.fit"),
            hole: p("demo::Seat.hole"),
            shaft: p("demo::Seat.shaft"),
            clearance: val(90, "0.09mm"),
            fit_type: "clearance".into(),
            required: None,
        });
        let other = comp(
            "demo::Pin.d",
            ComponentClass::Model,
            vec![param("demo::Pin.d", literal)],
        );
        project(vec![seat, other])
    }

    fn d002(proj: &Project, dcfg: &DConfig) -> Vec<Diagnostic> {
        check_d002(proj, &LintConfig::new(), dcfg, |_| unreachable!())
    }

    // frob:tests crates/cadetta-lint/src/rules/d.rs::check_d002
    #[test]
    fn d002_fires_for_fit_accessor_value() {
        // shaft is 12.010 -0.002, so mid = 12.009
        let d = d002(&fit_project(val(12_009, "12.009mm")), &DConfig::default());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "D002");
        assert!(d[0].message.contains("shaft.mid"), "{}", d[0].message);
        assert!(d002(&fit_project(val(12_500, "12.5mm")), &DConfig::default()).is_empty());
    }

    // frob:tests crates/cadetta-lint/src/rules/d.rs::check_d002
    #[test]
    fn d002_fires_for_table_cell_and_solver_output() {
        let cell = NamedValue {
            name: "Bearing.table[608].bore".into(),
            key: Key::new(&Dim::default(), &mm(8_000)),
        };
        let dcfg = DConfig {
            table_cells: vec![cell],
            ..DConfig::default()
        };
        let proj = project(vec![comp(
            "demo::P.d",
            ComponentClass::Model,
            vec![param("demo::P.d", val(8_000, "8mm"))],
        )]);
        assert_eq!(d002(&proj, &dcfg).len(), 1);

        let mut with_solver = project(vec![comp(
            "demo::P.d",
            ComponentClass::Model,
            vec![param("demo::P.d", val(7_777, "7.777mm")), {
                let mut m = param("demo::P.out", val(7_777, "7.777mm"));
                m.provenance = Provenance::Derived;
                m
            }],
        )]);
        with_solver.solvers.push(Solver {
            path: p("demo::S.run"),
            script: "s.py".into(),
            inputs: Default::default(),
            outputs: [("out".to_owned(), "demo::P.out".to_owned())].into(),
            deps: vec![],
            result_digest: None,
            stale: false,
        });
        let d = d002(&with_solver, &DConfig::default());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].path.as_deref(), Some("demo::P.d"));
    }

    // frob:ticket 01M4YPDTAGW415ZPEESX1PF271
    // frob:tests crates/cadetta-lint/src/rules/d.rs::is_literal
    #[test]
    fn table_lookup_is_not_a_literal() {
        assert!(is_literal(&param("demo::A.x", val(1000, "1mm"))));
        assert!(is_literal(&param("demo::A.x", val(-1000, "-1mm"))));
        assert!(!is_literal(&param(
            "demo::A.x",
            val(1000, "Iso273[6mm].pitch")
        )));
    }

    // frob:tests crates/cadetta-lint/src/rules/d.rs::check_d002
    #[test]
    fn d002_quiet_in_ots() {
        let mut proj = fit_project(val(12_009, "12.009mm"));
        proj.components[1].class = ComponentClass::Ots;
        assert!(d002(&proj, &DConfig::default()).is_empty());
    }

    fn reading(path: &str, dep: &str) -> Member {
        let mut m = Member::new(p(path), MemberKind::Param);
        m.provenance = Provenance::Derived;
        m.deps.push(p(dep));
        m
    }

    // frob:tests crates/cadetta-lint/src/rules/d.rs::check_d003
    #[test]
    fn d003_error_on_member_cycle() {
        let proj = project(vec![comp(
            "demo::A.x",
            ComponentClass::Model,
            vec![
                reading("demo::A.x", "demo::A.y"),
                reading("demo::A.y", "demo::A.x"),
            ],
        )]);
        let d = check_d003(&proj, |_| unreachable!());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "D003");
        assert_eq!(d[0].severity, Severity::Error);
        assert_eq!(LintConfig::new().severity(D003), Some(Severity::Error));
    }

    // frob:tests crates/cadetta-lint/src/rules/d.rs::check_d003
    #[test]
    fn d003_quiet_for_component_level_loop() {
        // A.x reads B.p and B.q reads A.y: components loop, members do not
        let proj = project(vec![
            comp(
                "demo::A.x",
                ComponentClass::Model,
                vec![
                    reading("demo::A.x", "demo::B.p"),
                    param("demo::A.y", val(1, "1mm")),
                ],
            ),
            comp(
                "demo::B.p",
                ComponentClass::Model,
                vec![
                    param("demo::B.p", val(2, "2mm")),
                    reading("demo::B.q", "demo::A.y"),
                ],
            ),
        ]);
        assert!(check_d003(&proj, |_| unreachable!()).is_empty());
    }
}
