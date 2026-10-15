//! Built-in SOLIDWORKS linked-equations adapter: emits and reads an equations
//! text file (emitters/sw-equations.md).
//!
//! Lines are `"NAME" = VALUE  ' comment`. The `'` header on line 1 is excluded
//! from change detection. Read accepts the same grammar plus expressions,
//! functions and `IIF` written by SOLIDWORKS itself.

use std::collections::{BTreeMap, BTreeSet};

use crate::policy::{EmitPoint, EmitRequest, PolicyError, to_decimal};
use crate::read_units::{NativeValue, ObservedValue, UnitPolicy};

/// Adapter name.
pub const ADAPTER: &str = "sw-equations";
/// Default decimal places (adapters.md 5).
pub const DEFAULT_PRECISION: u32 = 6;
/// Comment prefix of the header line.
const HEADER_PREFIX: &str = "' cadetta emit ";

/// The header comment fields: adapter version and the two digests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// Adapter name and version, e.g. `sw-equations 0.1.0`.
    pub adapter: String,
    /// IR digest, `blake3:<hex>`.
    pub ir: String,
    /// Bindings digest, `blake3:<hex>`.
    pub bindings: String,
}

/// A failure emitting or ordering an equations file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwEquationsError {
    /// A value failed emit policy (Q004).
    Policy(PolicyError),
    /// The dependency graph has a cycle through the named paths.
    Cycle(Vec<String>),
}

impl std::fmt::Display for SwEquationsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Policy(e) => write!(f, "{e}"),
            Self::Cycle(p) => write!(f, "dependency cycle through {}", p.join(", ")),
        }
    }
}

impl std::error::Error for SwEquationsError {}

impl From<PolicyError> for SwEquationsError {
    fn from(e: PolicyError) -> Self {
        Self::Policy(e)
    }
}

/// Kind of a parameter as far as the equations file cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// A dimensioned quantity: written with a unit suffix.
    Quantity,
    /// A Count: written bare.
    Count,
    /// A Ratio: written bare.
    Ratio,
    /// Text: never emitted to equations.
    Text,
}

/// Text of a written value: the emit point in the native unit at `precision`
/// decimals; `None` for Text; Q004 when not representable.
pub fn written_value(
    kind: ValueKind,
    req: &EmitRequest,
    precision: u32,
) -> Result<Option<String>, SwEquationsError> {
    match kind {
        ValueKind::Text => {
            tracing::debug!(path = req.path.as_str(), "text skipped in equations");
            Ok(None)
        }
        ValueKind::Count | ValueKind::Ratio => Ok(Some(to_decimal(req.value, precision)?)),
        ValueKind::Quantity => Ok(Some(format!(
            "{}{}",
            to_decimal(req.value, precision)?,
            req.unit
        ))),
    }
}

fn point_name(p: EmitPoint) -> &'static str {
    match p {
        EmitPoint::Nominal => "nominal",
        EmitPoint::Mid => "mid",
        EmitPoint::Mmc => "mmc",
        EmitPoint::Lmc => "lmc",
    }
}

/// Builds a trailing comment: the authored literal when its text differs from
/// `written`, then `emit=POINT` when not nominal, then the citation; empty if none.
pub fn trailing_comment(
    authored: Option<&str>,
    written: &str,
    point: EmitPoint,
    citation: Option<&str>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(a) = authored.filter(|a| *a != written) {
        parts.push(a.to_string());
    }
    if point != EmitPoint::Nominal {
        parts.push(format!("emit={}", point_name(point)));
    }
    if let Some(c) = citation {
        parts.push(c.to_string());
    }
    parts.join(" ")
}

/// Where a SOLIDWORKS dimension lives (emitters/sw-equations.md 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DimContext {
    /// A part dimension or sketch dimension: `.SLDPRT`.
    Part,
    /// An assembly-level dimension such as a mate distance: `.SLDASM`.
    Assembly,
    /// A part inside an assembly, named as the feature tree shows it (`wheel<2>`);
    /// the spelling is the part file's.
    Component(String),
}

/// Spells `Dn@Feature@File.EXT` for a dimension in `context`.
pub fn dimension_locator(dim: &str, feature: &str, file: &str, context: &DimContext) -> String {
    let ext = match context {
        DimContext::Assembly => "SLDASM",
        DimContext::Part | DimContext::Component(_) => "SLDPRT",
    };
    format!("{dim}@{feature}@{file}.{ext}")
}

/// The right-hand side of an emitted line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rhs {
    /// A written value such as `6.35mm` or `4`.
    Value(String),
    /// A reference to a global by name (ref mode).
    Ref(String),
}

/// One emitted equation line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Left-hand name: a global, or a `Dn@Feature@File` locator.
    pub lhs: String,
    /// Right-hand side.
    pub rhs: Rhs,
    /// Trailing comment text; may be empty.
    pub comment: String,
}

/// All lines of one path: its doc comment, then globals, then dimensions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathBlock {
    /// Canonical path.
    pub path: String,
    /// Doc comment, one entry per source line.
    pub doc: Vec<String>,
    /// Global lines in bindings-file order.
    pub globals: Vec<Line>,
    /// Dimension lines in bindings-file order.
    pub dimensions: Vec<Line>,
}

/// Orders paths topologically, always taking the smallest ready canonical path.
///
/// `graph` maps every path (emitted or not) to its direct dependencies; unknown
/// dependencies are ignored. Returns only the paths in `emitted`, so dependencies
/// count transitively through unemitted paths.
pub fn order_paths(
    graph: &BTreeMap<String, Vec<String>>,
    emitted: &BTreeSet<String>,
) -> Result<Vec<String>, SwEquationsError> {
    let mut placed: BTreeSet<&str> = BTreeSet::new();
    let mut out = Vec::new();
    while placed.len() < graph.len() {
        let next = graph.iter().find(|(p, deps)| {
            !placed.contains(p.as_str())
                && deps
                    .iter()
                    .all(|d| !graph.contains_key(d) || placed.contains(d.as_str()))
        });
        let Some((p, _)) = next else {
            let stuck: Vec<String> = graph
                .keys()
                .filter(|p| !placed.contains(p.as_str()))
                .cloned()
                .collect();
            tracing::warn!(paths = ?stuck, "equations dependency cycle");
            return Err(SwEquationsError::Cycle(stuck));
        };
        placed.insert(p.as_str());
        if emitted.contains(p) {
            out.push(p.clone());
        }
    }
    tracing::debug!(paths = out.len(), "equations paths ordered");
    Ok(out)
}

fn render_line(l: &Line, s: &mut String) {
    s.push_str(&format!("\"{}\" = ", l.lhs));
    match &l.rhs {
        Rhs::Value(v) => s.push_str(v),
        Rhs::Ref(g) => s.push_str(&format!("\"{g}\"")),
    }
    if !l.comment.is_empty() {
        s.push_str("  ' ");
        s.push_str(&l.comment);
    }
    s.push('\n');
}

/// Serialises an equations file: header, then each block in the given order, LF.
pub fn emit(header: &Header, blocks: &[PathBlock]) -> String {
    let mut s = format!(
        "{HEADER_PREFIX}{} ir={} bindings={}\n",
        header.adapter, header.ir, header.bindings
    );
    for b in blocks {
        for d in &b.doc {
            if d.is_empty() {
                s.push_str("'\n");
            } else {
                s.push_str(&format!("' {d}\n"));
            }
        }
        for l in b.globals.iter().chain(&b.dimensions) {
            render_line(l, &mut s);
        }
    }
    tracing::debug!(
        blocks = blocks.len(),
        bytes = s.len(),
        "sw equations emitted"
    );
    s
}

/// Functions SOLIDWORKS equations define; any other call is not parseable (kept raw).
const KNOWN_FUNCS: &[&str] = &[
    "sin", "cos", "tan", "sec", "cosec", "cotan", "arcsin", "arccos", "atn", "arcsec", "arccosec",
    "arccotan", "abs", "exp", "log", "sqr", "int", "sgn", "iif",
];

/// A parsed equation expression (observed IR).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// A number with its resolved unit.
    Num(ObservedValue),
    /// A reference to a global or dimension by name.
    Name(String),
    /// Unary minus.
    Neg(Box<Expr>),
    /// A binary operator: `+ - * / ^ = <> < > <= >=`.
    Binary {
        /// Operator text.
        op: String,
        /// Left operand.
        lhs: Box<Expr>,
        /// Right operand.
        rhs: Box<Expr>,
    },
    /// A function call, including `IIF`.
    Call {
        /// Function name as written.
        func: String,
        /// Arguments.
        args: Vec<Expr>,
    },
}

/// The right-hand side of a line as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadRhs {
    /// A plain number, unit resolved by the policy.
    Value(ObservedValue),
    /// An expression, function call or reference.
    Expr(Expr),
    /// Unparseable; `literal` holds the raw text and `exact` is false.
    Unparsed(ObservedValue),
}

/// One line as read back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadLine {
    /// Left-hand name; `None` when the line is wholly unparseable.
    pub name: Option<String>,
    /// Right-hand side.
    pub rhs: ReadRhs,
    /// Trailing comment, when present.
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(String, Option<String>),
    Str(String),
    Ident(String),
    Op(String),
    LParen,
    RParen,
    Comma,
}

fn lex(s: &str) -> Option<Vec<Tok>> {
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < cs.len() {
        let c = cs[i];
        match c {
            ' ' | '\t' | '\r' => i += 1,
            '"' => {
                let j = cs[i + 1..].iter().position(|&x| x == '"')? + i + 1;
                out.push(Tok::Str(cs[i + 1..j].iter().collect()));
                i = j + 1;
            }
            '0'..='9' | '.' => {
                let st = i;
                while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.') {
                    i += 1;
                }
                let lit: String = cs[st..i].iter().collect();
                let us = i;
                while i < cs.len() && cs[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let unit: String = cs[us..i].iter().collect();
                out.push(Tok::Num(lit, (!unit.is_empty()).then_some(unit)));
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let st = i;
                while i < cs.len() && (cs[i].is_ascii_alphanumeric() || cs[i] == '_') {
                    i += 1;
                }
                out.push(Tok::Ident(cs[st..i].iter().collect()));
            }
            '(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            '+' | '-' | '*' | '/' | '^' | '=' => {
                out.push(Tok::Op(c.to_string()));
                i += 1;
            }
            '<' | '>' => {
                let two = cs.get(i + 1).filter(|n| matches!(n, '=' | '>'));
                match (c, two) {
                    (_, Some(n)) if c == '>' && *n == '>' => return None,
                    (_, Some(n)) => {
                        out.push(Tok::Op(format!("{c}{n}")));
                        i += 2;
                    }
                    _ => {
                        out.push(Tok::Op(c.to_string()));
                        i += 1;
                    }
                }
            }
            _ => return None,
        }
    }
    Some(out)
}

struct Parser<'a> {
    toks: Vec<Tok>,
    pos: usize,
    units: &'a UnitPolicy,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn op(&mut self, ops: &[&str]) -> Option<String> {
        if let Some(Tok::Op(o)) = self.peek()
            && ops.contains(&o.as_str())
        {
            let o = o.clone();
            self.pos += 1;
            return Some(o);
        }
        None
    }

    fn binary(op: String, lhs: Expr, rhs: Expr) -> Expr {
        Expr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    fn cmp(&mut self) -> Option<Expr> {
        let l = self.add()?;
        if let Some(op) = self.op(&["=", "<>", "<", ">", "<=", ">="]) {
            let r = self.add()?;
            return Some(Self::binary(op, l, r));
        }
        Some(l)
    }

    fn add(&mut self) -> Option<Expr> {
        let mut l = self.mul()?;
        while let Some(op) = self.op(&["+", "-"]) {
            l = Self::binary(op, l, self.mul()?);
        }
        Some(l)
    }

    fn mul(&mut self) -> Option<Expr> {
        let mut l = self.unary()?;
        while let Some(op) = self.op(&["*", "/"]) {
            l = Self::binary(op, l, self.unary()?);
        }
        Some(l)
    }

    fn unary(&mut self) -> Option<Expr> {
        if self.op(&["-"]).is_some() {
            return Some(Expr::Neg(Box::new(self.unary()?)));
        }
        let base = self.atom()?;
        if let Some(op) = self.op(&["^"]) {
            return Some(Self::binary(op, base, self.unary()?));
        }
        Some(base)
    }

    fn atom(&mut self) -> Option<Expr> {
        let t = self.toks.get(self.pos)?.clone();
        self.pos += 1;
        match t {
            Tok::Num(literal, unit) => {
                Some(Expr::Num(self.units.read(&NativeValue { literal, unit })))
            }
            Tok::Str(n) => Some(Expr::Name(n)),
            Tok::LParen => {
                let e = self.cmp()?;
                matches!(self.toks.get(self.pos), Some(Tok::RParen)).then(|| self.pos += 1)?;
                Some(e)
            }
            Tok::Ident(func) => {
                if !KNOWN_FUNCS.contains(&func.to_ascii_lowercase().as_str()) {
                    tracing::warn!(func = func.as_str(), "unknown equation function");
                    return None;
                }
                matches!(self.toks.get(self.pos), Some(Tok::LParen)).then(|| self.pos += 1)?;
                let mut args = Vec::new();
                if matches!(self.peek(), Some(Tok::RParen)) {
                    self.pos += 1;
                    return Some(Expr::Call { func, args });
                }
                loop {
                    args.push(self.cmp()?);
                    match self.toks.get(self.pos)? {
                        Tok::Comma => self.pos += 1,
                        Tok::RParen => {
                            self.pos += 1;
                            return Some(Expr::Call { func, args });
                        }
                        _ => return None,
                    }
                }
            }
            _ => None,
        }
    }
}

fn parse_rhs(text: &str, units: &UnitPolicy) -> Option<ReadRhs> {
    let toks = lex(text)?;
    if let [Tok::Num(literal, unit)] = toks.as_slice() {
        return Some(ReadRhs::Value(units.read(&NativeValue {
            literal: literal.clone(),
            unit: unit.clone(),
        })));
    }
    let mut p = Parser {
        toks,
        pos: 0,
        units,
    };
    let e = p.cmp()?;
    (p.pos == p.toks.len()).then_some(ReadRhs::Expr(e))
}

/// Splits a line at the first `'` outside double quotes.
fn split_comment(line: &str) -> (&str, Option<&str>) {
    let mut in_q = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_q = !in_q,
            '\'' if !in_q => return (&line[..i], Some(line[i + 1..].trim())),
            _ => {}
        }
    }
    (line, None)
}

fn unparsed(raw: &str) -> ReadRhs {
    ReadRhs::Unparsed(ObservedValue {
        literal: raw.to_string(),
        unit: None,
        unit_assumed: false,
        exact: false,
        reason: Some("unparseable equation line".to_string()),
    })
}

fn read_line(line: &str, units: &UnitPolicy) -> ReadLine {
    let (code, comment) = split_comment(line);
    let comment = comment.filter(|c| !c.is_empty()).map(str::to_string);
    let code = code.trim();
    let parsed = code
        .strip_prefix('"')
        .and_then(|r| r.split_once('"'))
        .and_then(|(name, rest)| {
            let rhs = rest.trim_start().strip_prefix('=')?;
            Some((name.to_string(), parse_rhs(rhs, units)?))
        });
    match parsed {
        Some((name, rhs)) => ReadLine {
            name: Some(name),
            rhs,
            comment,
        },
        None => {
            tracing::warn!(line, "equation line not parseable; kept raw");
            ReadLine {
                name: None,
                rhs: unparsed(line.trim()),
                comment: None,
            }
        }
    }
}

/// Reads an equations file; comment and blank lines are skipped, unparseable lines kept raw.
pub fn read(text: &str, units: &UnitPolicy) -> Vec<ReadLine> {
    let out: Vec<ReadLine> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('\''))
        .map(|l| read_line(l, units))
        .collect();
    tracing::debug!(lines = out.len(), "sw equations read");
    out
}

/// Compares two files ignoring the header line (the `sync` no-op comparison, adapters.md 2).
pub fn same_modulo_header(a: &str, b: &str) -> bool {
    fn body(t: &str) -> Vec<&str> {
        t.lines()
            .enumerate()
            .filter(|(i, l)| !(*i == 0 && l.starts_with(HEADER_PREFIX)))
            .map(|(_, l)| l)
            .collect()
    }
    body(a) == body(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::Ratio;

    fn header(ir: &str) -> Header {
        Header {
            adapter: "sw-equations 0.1.0".into(),
            ir: ir.into(),
            bindings: "blake3:bb".into(),
        }
    }

    fn req(num: i128, den: i128, unit: &str) -> EmitRequest {
        EmitRequest {
            path: "p".into(),
            point: EmitPoint::Nominal,
            value: Ratio::new(num, den),
            unit: unit.into(),
        }
    }

    fn line(lhs: &str, rhs: Rhs, comment: &str) -> Line {
        Line {
            lhs: lhs.into(),
            rhs,
            comment: comment.into(),
        }
    }

    fn blocks() -> Vec<PathBlock> {
        let thk = written_value(ValueKind::Quantity, &req(127, 20, "mm"), 6)
            .unwrap()
            .unwrap();
        let bore = written_value(ValueKind::Quantity, &req(12009, 1000, "mm"), 6)
            .unwrap()
            .unwrap();
        vec![
            PathBlock {
                path: "a::Plate.thk".into(),
                doc: vec!["Stock thickness, 1/4 in nominal.".into()],
                globals: vec![line(
                    "Plate_Thk",
                    Rhs::Value(thk.clone()),
                    &trailing_comment(
                        Some("1/4in"),
                        &thk,
                        EmitPoint::Nominal,
                        Some("@\"ASTM B209\""),
                    ),
                )],
                dimensions: vec![line(
                    "D1@Boss-Extrude1@CoverPlate.SLDPRT",
                    Rhs::Ref("Plate_Thk".into()),
                    "",
                )],
            },
            PathBlock {
                path: "a::Plate.bore".into(),
                doc: vec![],
                globals: vec![
                    line(
                        "PilotBore",
                        Rhs::Value(bore.clone()),
                        &trailing_comment(Some("12mm H7"), &bore, EmitPoint::Mid, None),
                    ),
                    line("NumBolts", Rhs::Value("4".into()), ""),
                ],
                dimensions: vec![],
            },
        ]
    }

    const GOLDEN: &str = "' cadetta emit sw-equations 0.1.0 ir=blake3:aa bindings=blake3:bb\n\
' Stock thickness, 1/4 in nominal.\n\
\"Plate_Thk\" = 6.35mm  ' 1/4in @\"ASTM B209\"\n\
\"D1@Boss-Extrude1@CoverPlate.SLDPRT\" = \"Plate_Thk\"\n\
\"PilotBore\" = 12.009mm  ' 12mm H7 emit=mid\n\
\"NumBolts\" = 4\n";

    // frob:tests crates/cadetta-adapter/src/builtin/sw_equations.rs::emit kind=unit
    #[test]
    fn emit_is_byte_identical_golden() {
        let a = emit(&header("blake3:aa"), &blocks());
        assert_eq!(a, emit(&header("blake3:aa"), &blocks()));
        assert_eq!(a, GOLDEN);
        assert!(
            a.starts_with("' cadetta emit sw-equations 0.1.0 ir=blake3:aa bindings=blake3:bb\n")
        );
    }

    // frob:tests crates/cadetta-adapter/src/builtin/sw_equations.rs::written_value kind=unit
    #[test]
    fn values_kinds_and_q004() {
        let q = |k, r: &EmitRequest| written_value(k, r, 6).unwrap();
        assert_eq!(
            q(ValueKind::Quantity, &req(1, 4, "in")),
            Some("0.25in".into())
        );
        assert_eq!(q(ValueKind::Quantity, &req(5, 1, "mm")), Some("5mm".into()));
        assert_eq!(q(ValueKind::Count, &req(4, 1, "")), Some("4".into()));
        assert_eq!(q(ValueKind::Ratio, &req(1, 2, "")), Some("0.5".into()));
        assert_eq!(q(ValueKind::Text, &req(1, 2, "")), None);
        let e = written_value(ValueKind::Quantity, &req(1, 3, "mm"), 6).unwrap_err();
        assert!(matches!(
            e,
            SwEquationsError::Policy(PolicyError::Inexact { .. })
        ));
    }

    // frob:tests crates/cadetta-adapter/src/builtin/sw_equations.rs::trailing_comment kind=unit
    #[test]
    fn comment_content_and_order() {
        let c = |a, w, p, cite| trailing_comment(a, w, p, cite);
        assert_eq!(c(Some("5mm"), "5mm", EmitPoint::Nominal, None), "");
        assert_eq!(c(None, "5mm", EmitPoint::Nominal, None), "");
        assert_eq!(
            c(Some("12mm H7"), "12.009mm", EmitPoint::Mid, Some("@\"X\"")),
            "12mm H7 emit=mid @\"X\""
        );
        assert_eq!(
            c(None, "5mm", EmitPoint::Lmc, Some("@\"X\"")),
            "emit=lmc @\"X\""
        );
    }

    // frob:tests crates/cadetta-adapter/src/builtin/sw_equations.rs::emit kind=unit
    #[test]
    fn doc_comment_lines_precede_first_line() {
        let mut b = blocks();
        b[0].doc = vec!["one".into(), String::new(), "two".into()];
        let s = emit(&header("h"), &b);
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(&lines[1..4], ["' one", "'", "' two"]);
        assert!(lines[4].starts_with("\"Plate_Thk\""));
    }

    // frob:tests crates/cadetta-adapter/src/builtin/sw_equations.rs::order_paths kind=unit
    #[test]
    fn order_is_topological_ties_by_path() {
        let g: BTreeMap<String, Vec<String>> = [
            ("a", vec!["c"]),
            ("b", vec![]),
            ("c", vec!["z"]),
            ("z", vec![]),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.into_iter().map(String::from).collect()))
        .collect();
        let all: BTreeSet<String> = g.keys().cloned().collect();
        assert_eq!(order_paths(&g, &all).unwrap(), ["b", "z", "c", "a"]);
        // `c` is unemitted but still orders `a` after `z`.
        let some: BTreeSet<String> = ["a", "z"].iter().map(|s| s.to_string()).collect();
        assert_eq!(order_paths(&g, &some).unwrap(), ["z", "a"]);
        let cyc: BTreeMap<String, Vec<String>> = [
            ("a".to_string(), vec!["b".to_string()]),
            ("b".to_string(), vec!["a".to_string()]),
        ]
        .into_iter()
        .collect();
        assert!(matches!(
            order_paths(&cyc, &cyc.keys().cloned().collect()),
            Err(SwEquationsError::Cycle(_))
        ));
    }

    // frob:tests crates/cadetta-adapter/src/builtin/sw_equations.rs::read kind=unit
    #[test]
    fn read_round_trips_emitted_file() {
        let units = UnitPolicy {
            native_unit: Some("mm".into()),
            assume_unit: None,
        };
        let got = read(GOLDEN, &units);
        assert_eq!(got.len(), 4);
        assert_eq!(got[0].name.as_deref(), Some("Plate_Thk"));
        assert_eq!(got[0].comment.as_deref(), Some("1/4in @\"ASTM B209\""));
        let ReadRhs::Value(v) = &got[0].rhs else {
            panic!("value")
        };
        assert_eq!(
            (v.literal.as_str(), v.unit.as_deref()),
            ("6.35", Some("mm"))
        );
        assert!(!v.unit_assumed);
        assert_eq!(got[1].rhs, ReadRhs::Expr(Expr::Name("Plate_Thk".into())));
        let ReadRhs::Value(n) = &got[3].rhs else {
            panic!("value")
        };
        assert!(n.unit_assumed);
    }

    // frob:tests crates/cadetta-adapter/src/builtin/sw_equations.rs::read kind=unit
    #[test]
    fn read_expressions_functions_and_iif() {
        let u = UnitPolicy::default();
        let got = read("\"A\" = \"B\" * 2\n\"C\" = IIF(\"A\" > 3, 1, sin(2))\n", &u);
        let ReadRhs::Expr(Expr::Binary { op, lhs, .. }) = &got[0].rhs else {
            panic!("binary")
        };
        assert_eq!(op, "*");
        assert_eq!(**lhs, Expr::Name("B".into()));
        let ReadRhs::Expr(Expr::Call { func, args }) = &got[1].rhs else {
            panic!("call")
        };
        assert_eq!((func.as_str(), args.len()), ("IIF", 3));
    }

    // frob:ticket 01M50J83ATKTE83FQVVG0V5V7D
    // frob:tests crates/cadetta-adapter/src/builtin/sw_equations.rs::read kind=unit
    #[test]
    fn read_keeps_unknown_function_call_raw() {
        let got = read("\"A\" = SWFN(1)\n", &UnitPolicy::default());
        assert!(matches!(&got[0].rhs, ReadRhs::Unparsed(o) if !o.exact));
        let ok = read("\"A\" = sqr(4)\n", &UnitPolicy::default());
        assert!(matches!(&ok[0].rhs, ReadRhs::Expr(Expr::Call { .. })));
    }

    // frob:tests crates/cadetta-adapter/src/builtin/sw_equations.rs::read kind=unit
    #[test]
    fn read_keeps_unparseable_lines_raw() {
        let got = read("\"A\" = 2 *\nwhat is this\n", &UnitPolicy::default());
        for g in &got {
            let ReadRhs::Unparsed(o) = &g.rhs else {
                panic!("unparsed")
            };
            assert!(!o.exact);
            assert!(o.reason.is_some());
        }
        assert_eq!(got.len(), 2);
        assert!(got[0].name.is_none());
        let ReadRhs::Unparsed(o) = &got[1].rhs else {
            panic!()
        };
        assert_eq!(o.literal, "what is this");
    }

    // frob:tests crates/cadetta-adapter/src/builtin/sw_equations.rs::same_modulo_header kind=unit
    #[test]
    fn header_is_excluded_from_comparison() {
        let a = emit(&header("blake3:aa"), &blocks());
        let b = emit(&header("blake3:cc"), &blocks());
        assert_ne!(a, b);
        assert!(same_modulo_header(&a, &b));
        assert!(!same_modulo_header(&a, &a.replace("= 4", "= 5")));
    }

    // frob:tests crates/cadetta-adapter/src/builtin/sw_equations.rs::dimension_locator kind=unit
    #[test]
    fn solidworks_naming() {
        let c = DimContext::Component("wheel<2>".into());
        assert_eq!(
            dimension_locator("D1", "Boss1", "Wheel", &DimContext::Part),
            "D1@Boss1@Wheel.SLDPRT"
        );
        assert_eq!(
            dimension_locator("D1", "Boss1", "Wheel", &c),
            "D1@Boss1@Wheel.SLDPRT"
        );
        assert_eq!(
            dimension_locator("D2", "Mate1", "Rover", &DimContext::Assembly),
            "D2@Mate1@Rover.SLDASM"
        );
    }
}
