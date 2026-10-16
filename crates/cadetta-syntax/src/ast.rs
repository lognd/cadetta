//! The syntax tree for .cdt files (language.md 3 and 5): every node carries a [`Span`].
//!
//! Numbers and quantity literals stay as source text; sema converts them to
//! exact rationals, so no float is ever built here.

// frob:ticket 01M4SPWP1JK8XH8EKXK71P7EFH

pub use crate::lexer::{Segment, Span};

/// An identifier with its span.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Ident {
    /// The identifier text.
    pub name: String,
    /// Where it is.
    pub span: Span,
}

/// The separator between two path segments.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PathSep {
    /// `::`
    ColonColon,
    /// `.`
    Dot,
}

/// A path such as `Material::Al6061T6` or `acme.old::Bracket.thickness`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Path {
    /// The segments, at least one.
    pub segments: Vec<Ident>,
    /// The separator after each segment but the last (`segments.len() - 1` entries).
    pub seps: Vec<PathSep>,
    /// Where it is.
    pub span: Span,
}

impl Path {
    /// The segment names joined with `::` or `.` as written.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for (i, seg) in self.segments.iter().enumerate() {
            if i > 0 {
                out.push_str(match self.seps[i - 1] {
                    PathSep::ColonColon => "::",
                    PathSep::Dot => ".",
                });
            }
            out.push_str(&seg.name);
        }
        out
    }
}

/// A `///` doc comment line attached to an item or member.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Doc {
    /// The prose after `///`.
    pub text: String,
    /// Where it is.
    pub span: Span,
}

/// An attribute argument.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AttrArg {
    /// A positional expression (`eol`, `"2027-06"`).
    Expr(Expr),
    /// A named argument (`Iface = Impl`).
    Named {
        /// The name.
        name: Ident,
        /// The value.
        value: Expr,
    },
}

/// One attribute inside `#[...]` or `#![...]`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Attr {
    /// The attribute name.
    pub name: Ident,
    /// Arguments; empty when written without parentheses.
    pub args: Vec<AttrArg>,
    /// True for the `#![...]` inner form.
    pub inner: bool,
    /// Where it is.
    pub span: Span,
}

// frob:ticket 01M4VB62VJQV9AKSSQME8NWW6Y
/// A citation `@path`, `@"literal"` with an optional `:"locator"` (language.md 3.9).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Citation {
    /// The cite name or the literal text.
    pub name: CiteName,
    /// The locator string, when given.
    pub locator: Option<String>,
    /// Where it is.
    pub span: Span,
}

/// What a citation names.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CiteName {
    /// `@path`, a `cite` item path or a pseudo-citation (`measured`, `assumed`, `estimate`).
    Named(Path),
    /// `@"literal"`, text that is never checked for reachability.
    Literal(String),
}

/// Item and `use` visibility (packages.md 3).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Vis {
    /// No modifier.
    #[default]
    Private,
    /// `pub`
    Pub,
    /// `pub(package)`
    PubPackage,
}

/// A parsed file: `{ use | item }` plus any file-level inner attributes.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct File {
    /// File-level `#![...]` attributes.
    pub inner_attrs: Vec<Attr>,
    /// Uses and items in source order.
    pub entries: Vec<FileEntry>,
    /// The whole file.
    pub span: Span,
}

impl File {
    /// The `use` declarations, in order.
    pub fn uses(&self) -> impl Iterator<Item = &Use> {
        self.entries.iter().filter_map(|e| match e {
            FileEntry::Use(u) => Some(u),
            FileEntry::Item(_) => None,
        })
    }

    /// The items, in order.
    pub fn items(&self) -> impl Iterator<Item = &Item> {
        self.entries.iter().filter_map(|e| match e {
            FileEntry::Item(i) => Some(&**i),
            FileEntry::Use(_) => None,
        })
    }

    /// The components (`ots`, `model`, `assembly`), in order.
    pub fn components(&self) -> impl Iterator<Item = &Component> {
        self.items().filter_map(|i| match &i.kind {
            ItemKind::Component(c) => Some(c),
            _ => None,
        })
    }
}

/// One top-level entry.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FileEntry {
    /// A `use` declaration.
    Use(Use),
    /// Any other item.
    Item(Box<Item>),
}

/// A `use` declaration (language.md 3; packages.md 4).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Use {
    /// `pub`, `pub(package)` or none.
    pub vis: Vis,
    /// The imported path (for a tree or glob, the prefix).
    pub path: UsePath,
    /// What is imported from the path.
    pub kind: UseKind,
    /// Where it is.
    pub span: Span,
}

/// The root of a use path.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum UseRoot {
    /// `package`, this package's root.
    Package,
    /// `self`, the current module.
    SelfMod,
    /// `super`, repeated `n` times (`super::super` is 2).
    Super(usize),
    /// A dependency's dotted name or `std` (`acme.rover` is two parts).
    Pkg(Vec<Ident>),
}

/// A use path: a root and the `::` segments after it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UsePath {
    /// The root.
    pub root: UseRoot,
    /// The identifiers after the root.
    pub segments: Vec<Ident>,
    /// Where it is.
    pub span: Span,
}

/// What a `use` brings in.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum UseKind {
    /// The path itself, optionally renamed with `as`.
    Single {
        /// The `as` rename.
        alias: Option<Ident>,
    },
    /// `path::*`
    Glob,
    /// `path::{a, b as c, d::{...}}`
    Tree(Vec<UseTree>),
}

/// One entry of a use tree.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UseTree {
    /// The imported name.
    pub name: Ident,
    /// Rename, glob or nested group below the name.
    pub kind: UseTreeKind,
    /// Where it is.
    pub span: Span,
}

/// The shape of a [`UseTree`] entry.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum UseTreeKind {
    /// `name` or `name as alias`.
    Leaf {
        /// The `as` rename.
        alias: Option<Ident>,
    },
    /// `name::*`
    Glob,
    /// `name::{...}`
    Group(Vec<UseTree>),
    /// A bare `*` inside a group: a glob of the group's own prefix (`name` is `*`).
    Star,
}

/// An item with its docs, attributes and visibility.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Item {
    /// Doc comments.
    pub docs: Vec<Doc>,
    /// Leading attributes.
    pub attrs: Vec<Attr>,
    /// Visibility.
    pub vis: Vis,
    /// What the item is.
    pub kind: ItemKind,
    /// Where it is, docs and attributes excluded.
    pub span: Span,
}

/// The item kinds this parser reads.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ItemKind {
    /// `cite name = ...`
    Cite(Cite),
    /// `interface ...`
    Interface(Interface),
    /// `ots`, `model` or `assembly`.
    Component(Component),
    /// A file-scope `fit`.
    Fit(Fit),
    /// A file-scope `require`.
    Require(Require),
    /// A file-scope `check`.
    Check(Check),
    /// `process NAME { ... }`
    Process(Process),
    /// `table NAME { ... }`
    Table(Table),
    /// `enum NAME { A, B }`
    Enum(EnumDecl),
    /// `fn name(...) -> Kind { ... }`
    Fn(FnDecl),
    /// A file-scope `solver`.
    Solver(Solver),
    /// `waive CODE on GLOB { because ... }`
    Waive(Waive),
    /// `unit ...`
    Unit(UnitDecl),
    /// `kind NAME = kindexpr`
    Kind(KindDecl),
    /// An item whose keyword is recognised but not parsed yet, or that had a syntax error.
    Error,
}

/// A typed `cite NAME: TYPE { field = "value" ... }` item (language.md 3.9).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cite {
    /// The cite name.
    pub name: Ident,
    /// The declared type word (checked by sema, E006).
    pub ty: Ident,
    /// The fields in source order.
    pub fields: Vec<CiteFieldDecl>,
}

/// One `name = value` line of a `cite` body.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CiteFieldDecl {
    /// The field name (checked by sema, E006).
    pub name: Ident,
    /// The value as written.
    pub value: CiteValue,
    /// Where the whole field is.
    pub span: Span,
}

/// The value of a `cite` field.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CiteValue {
    /// A string.
    Text(String),
    /// A list of strings (only `author` accepts one).
    List(Vec<String>),
    /// Anything else (wrongly shaped; sema reports E006).
    Other,
}

/// An interface declaration.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Interface {
    /// The interface name.
    pub name: Ident,
    /// Extended interfaces.
    pub extends: Vec<Path>,
    /// Members in order.
    pub members: Vec<InterfaceMember>,
}

/// A member of an interface.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InterfaceMember {
    /// Doc comments.
    pub docs: Vec<Doc>,
    /// Leading attributes.
    pub attrs: Vec<Attr>,
    /// What it is.
    pub kind: InterfaceMemberKind,
    /// Where it is.
    pub span: Span,
}

/// The interface member kinds.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum InterfaceMemberKind {
    /// `name: Type` or `name?: Type`.
    Field {
        /// The member name.
        name: Ident,
        /// True when written with `?`.
        optional: bool,
        /// The kind.
        ty: Type,
    },
    /// `datum name`
    Datum(Datum),
    /// A bound every implementer must meet.
    Bound(Box<Bound>),
    /// A syntax error or a member not parsed yet.
    Error,
}

/// The component classes (language.md 3.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ComponentClass {
    /// `ots`
    Ots,
    /// `model`
    Model,
    /// `assembly`
    Assembly,
}

/// A parameter declaration `name[?]: Type [= default]`; `?` marks an optional argument (language.md 3.3).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ParamDecl {
    /// The parameter name.
    pub name: Ident,
    /// Its type.
    pub ty: Type,
    /// The default, when given.
    pub default: Option<Expr>,
    /// True for `name?: Type`: the argument may be omitted and is then absent.
    pub optional: bool,
    /// Where it is.
    pub span: Span,
}

/// An `ots`, `model` or `assembly`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Component {
    /// Which class.
    pub class: ComponentClass,
    /// The name.
    pub name: Ident,
    /// Parameters (empty when there is no `( )`).
    pub params: Vec<ParamDecl>,
    /// Implemented interfaces.
    pub implements: Vec<Path>,
    /// The component citation.
    pub citation: Option<Citation>,
    /// `#![...]` attributes at the top of the body.
    pub inner_attrs: Vec<Attr>,
    /// Members in order.
    pub members: Vec<Member>,
}

/// A component member.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Member {
    /// Doc comments.
    pub docs: Vec<Doc>,
    /// Leading attributes.
    pub attrs: Vec<Attr>,
    /// True when written with `priv`.
    pub private: bool,
    /// What it is.
    pub kind: MemberKind,
    /// A citation after a non-parameter statement (language.md 3.9); parameters carry theirs inside.
    pub citation: Option<Citation>,
    /// Trailing same-line `#[...]` attributes.
    pub trailing: Vec<Attr>,
    /// Where it is.
    pub span: Span,
}

/// The member kinds.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MemberKind {
    /// `name [: Type] = expr ...`
    Param(Param),
    /// `let name = expr`
    Let(Let),
    /// `name <= expr` and the other bound forms.
    Bound(Box<Bound>),
    /// `part ...`
    Part(Part),
    /// `fit ...`
    Fit(Fit),
    /// `check ...`
    Check(Check),
    /// `require ...`
    Require(Require),
    /// `solver ...`
    Solver(Solver),
    /// `datum name = expr`
    Datum(Datum),
    /// `place part at expr`
    Place(Place),
    /// A syntax error or a member not parsed yet.
    Error,
}

/// A datum declaration `datum name [= expr]` (geometry.md 2); no value in an interface.
// frob:ticket 01M4SWFX0C2BXQGR8WQ2VN1G88
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Datum {
    /// The datum name.
    pub name: Ident,
    /// The defining expression (`origin.offset(..)`), absent for an interface datum.
    pub value: Option<Expr>,
    /// Where it is.
    pub span: Span,
}

/// A placement `place target at expr` (geometry.md 4).
// frob:ticket 01M4SWFX0C2BXQGR8WQ2VN1G88
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Place {
    /// The part datum being placed (`rail.origin`, `feet[2].top`, `feet.top`).
    pub target: Expr,
    /// The frame it is made to coincide with (`base.site.offset(base.sites)`).
    pub at: Expr,
    /// Where it is.
    pub span: Span,
}

/// A parameter `name [: Type] = expr [citation] { was path }`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Param {
    /// The name.
    pub name: Ident,
    /// The declared kind, when given.
    pub ty: Option<Type>,
    /// The value.
    pub value: Expr,
    /// Provenance.
    pub citation: Option<Citation>,
    /// Old paths from `was`.
    pub was: Vec<Path>,
}

/// An analysis value `let name [: Type] = expr`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Let {
    /// The name.
    pub name: Ident,
    /// The declared kind, when given.
    pub ty: Option<Type>,
    /// The value.
    pub value: Expr,
    /// Where it is.
    pub span: Span,
}

/// A bound (language.md 3.4).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Bound {
    /// Which form.
    pub kind: BoundKind,
    /// Where it is.
    pub span: Span,
}

/// The bound forms.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BoundKind {
    /// `name <= expr`
    Le {
        /// The bounded member.
        name: Ident,
        /// The limit.
        value: Expr,
    },
    /// `name >= expr`
    Ge {
        /// The bounded member.
        name: Ident,
        /// The limit.
        value: Expr,
    },
    /// `name in lo..hi`
    In {
        /// The bounded member.
        name: Ident,
        /// The lower end.
        lo: Expr,
        /// The upper end.
        hi: Expr,
    },
    /// `path == path`
    Eq {
        /// Left side.
        lhs: Path,
        /// Right side.
        rhs: Path,
    },
}

/// A part slot `part name[N]: Type(args) = Primary alt [Alt, ...]`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Part {
    /// The slot name.
    pub name: Ident,
    /// The `[N]` count expression of an array slot.
    pub count: Option<Expr>,
    /// The slot type.
    pub ty: Type,
    /// Arguments written after the type (`part axle: Axle(wheel = w)`).
    pub args: Option<Vec<Arg>>,
    /// The primary implementer after `=`.
    pub primary: Option<Ctor>,
    /// Approved alternates from `alt [...]`.
    pub alts: Vec<Ctor>,
}

/// A constructor `path` or `path(args)`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Ctor {
    /// The implementer.
    pub path: Path,
    /// Arguments, when parenthesised.
    pub args: Option<Vec<Arg>>,
    /// Where it is.
    pub span: Span,
}

/// A call or constructor argument, positional or `name = value`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Arg {
    /// The argument name for the named form.
    pub name: Option<Ident>,
    /// The value.
    pub value: Expr,
    /// A citation after the value (language.md 3.9).
    pub citation: Option<Citation>,
    /// Where it is.
    pub span: Span,
}

/// A fit (language.md 3.7).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Fit {
    /// The fit name.
    pub name: Ident,
    /// The class or path pair.
    pub spec: FitSpec,
    /// The path the fit is on; for an `a / b` fit written without `on` this is the hole path `a`.
    pub on: Path,
    /// True when `on` was not written (only possible for an `a / b` path pair).
    pub on_implied: bool,
    /// The required range `[lo .. hi]`.
    pub range: Option<(Expr, Expr)>,
}

/// The fit specification.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FitSpec {
    /// One class (`p6`, `H7`).
    Class(Ident),
    /// A hole/shaft class pair (`H7/p6`).
    Pair(Ident, Ident),
    /// Two paths (`hole / Insert.od`).
    Paths(Path, Path),
}

/// `check NAME [for x in expr]: expr [message]`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Check {
    /// The check name.
    pub name: Ident,
    /// The quantifier `for x in expr`.
    pub each: Option<(Ident, Expr)>,
    /// The condition.
    pub cond: Expr,
    /// The optional message.
    pub message: Option<String>,
}

/// `require NAME: expr [message] [citation]`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Require {
    /// The requirement name.
    pub name: Ident,
    /// The condition.
    pub cond: Expr,
    /// The optional message.
    pub message: Option<String>,
    /// The external requirement cited.
    pub citation: Option<Citation>,
}

/// A type (language.md 3.3).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Type {
    /// Which form.
    pub kind: TypeKind,
    /// Where it is.
    pub span: Span,
}

/// The type forms.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TypeKind {
    /// A kind, interface, enum or component name.
    Path(Path),
    /// `Point2 on datum`.
    OnDatum {
        /// The point type.
        base: Path,
        /// The datum path.
        datum: Path,
    },
    /// `[T; N]`
    Array {
        /// Element type.
        elem: Box<Type>,
        /// Length expression.
        len: Box<Expr>,
    },
    /// `Table<K>`
    Table(Box<Type>),
    /// A compound kind expression such as `Volume / Time` or `Pressure / VolumeFlow^2`.
    KindExpr(KindExpr),
}

/// Whether a kind-expression term multiplies or divides (language.md 3.10).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KindOp {
    /// `*`, also the implicit operator of the first term.
    Mul,
    /// `/`
    Div,
}

/// One factor of a kind expression: a kind name with an optional integer exponent.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KindTerm {
    /// How it combines with the terms before it.
    pub op: KindOp,
    /// The kind.
    pub name: Path,
    /// The `^n` exponent as written, when given.
    pub exp: Option<i32>,
    /// Where it is.
    pub span: Span,
}

/// A product and quotient of kinds, read left to right.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KindExpr {
    /// The terms, at least two for a compound expression.
    pub terms: Vec<KindTerm>,
}

/// A rule `rule name(param) -> Kind = expr` of a process (language.md 3.8).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Rule {
    /// The role name.
    pub name: Ident,
    /// The one parameter.
    pub param: ParamDecl,
    /// The result kind.
    pub ret: Type,
    /// The compensation expression.
    pub body: Expr,
}

/// A member of a process.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProcessMember {
    /// Doc comments.
    pub docs: Vec<Doc>,
    /// Leading attributes.
    pub attrs: Vec<Attr>,
    /// What it is.
    pub kind: ProcessMemberKind,
    /// Where it is.
    pub span: Span,
}

/// The process member kinds.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ProcessMemberKind {
    /// A process parameter such as `hole_comp = 0.2mm [+-0.05mm]`.
    Param(Box<Param>),
    /// A compensation rule.
    Rule(Box<Rule>),
    /// A syntax error.
    Error,
}

/// `process NAME [citation] { ... }`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Process {
    /// The process name.
    pub name: Ident,
    /// Provenance of the calibration.
    pub citation: Option<Citation>,
    /// Parameters and rules in order.
    pub members: Vec<ProcessMember>,
}

/// One table row; `values` is `None` for a key-only row (language.md 3.6).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TableRow {
    /// Doc comments.
    pub docs: Vec<Doc>,
    /// The key.
    pub key: Expr,
    /// The column values after `=>`, or `None` when the row has no `=>`.
    pub values: Option<Vec<Expr>>,
    /// Where it is.
    pub span: Span,
}

/// `table NAME [citation] { key ... cols ... rows }`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Table {
    /// The table name.
    pub name: Ident,
    /// Provenance of the data.
    pub citation: Option<Citation>,
    /// The key column; `None` only after a syntax error.
    pub key: Option<ParamDecl>,
    /// The value columns (empty for a key-only table).
    pub cols: Vec<ParamDecl>,
    /// The rows in order.
    pub rows: Vec<TableRow>,
}

/// `enum NAME { A, B }`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EnumDecl {
    /// The enum name.
    pub name: Ident,
    /// The variants in order.
    pub variants: Vec<Ident>,
}

/// `fn name(params) -> Kind { let ...; expr }`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FnDecl {
    /// The function name.
    pub name: Ident,
    /// Parameters.
    pub params: Vec<ParamDecl>,
    /// The result kind.
    pub ret: Type,
    /// The body.
    pub body: Block,
}

/// A solver input binding `name = expr`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Bind {
    /// The input name.
    pub name: Ident,
    /// The bound expression.
    pub value: Expr,
    /// Where it is.
    pub span: Span,
}

/// A statement of a solver body.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SolverMember {
    /// What it is.
    pub kind: SolverMemberKind,
    /// Where it is.
    pub span: Span,
}

/// The solver body statements (solvers.md 1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SolverMemberKind {
    /// `in a = expr, b = expr`
    In(Vec<Bind>),
    /// `out name: Kind`
    Out {
        /// The output name.
        name: Ident,
        /// Its kind or kind expression; `None` when the `: Kind` is missing (S007).
        ty: Option<Type>,
    },
    /// `deps "file"`
    Deps(String),
    /// `check name: expr` on the solver's own outputs (solvers.md 5)
    Check(Box<Check>),
    /// A syntax error.
    Error,
}

// frob:ticket 01M4VB630NZKXA6Q9D7253FATR
/// `solver NAME [@cite] { script "path" ... }` (solvers.md 1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Solver {
    /// The solver name.
    pub name: Ident,
    /// The optional citation of the method (not the script, never an input).
    pub citation: Option<Citation>,
    /// The script path from the `script` statement.
    pub script: String,
    /// Where the `script` statement is.
    pub script_span: Span,
    /// The body statements.
    pub members: Vec<SolverMember>,
}

/// Source text kept verbatim: a waiver glob, slot or alternate, or a date.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RawText {
    /// The text as written.
    pub text: String,
    /// Where it is.
    pub span: Span,
}

/// The `for alt SLOT=ALT` scope of a waiver (language.md 3.2).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WaiveAlt {
    /// The canonical slot path.
    pub slot: RawText,
    /// The qualified alternate.
    pub alt: RawText,
}

/// `waive CODE on GLOB [for alt SLOT=ALT] { because STRING [expires DATE] }` (lints.md 2.1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Waive {
    /// The diagnostic code.
    pub code: Ident,
    /// The path glob (obligations.md 2.2).
    pub glob: RawText,
    /// The alternate scope, when given.
    pub for_alt: Option<WaiveAlt>,
    /// The required reason.
    pub because: String,
    /// The expiry date `YYYY-MM-DD`, when given.
    pub expires: Option<RawText>,
}

/// `unit` declarations (language.md 3.10).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum UnitDecl {
    /// `unit kip = 1000lbf`
    Define {
        /// The new unit.
        name: Ident,
        /// Its value.
        value: Expr,
    },
    /// `unit Stress MPa`, the default unit of a kind.
    Default {
        /// The kind.
        kind: Ident,
        /// Its default unit.
        unit: Ident,
    },
}

/// `kind NAME = kindexpr`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KindDecl {
    /// The new kind.
    pub name: Ident,
    /// What it equals: a kind name or a kind expression.
    pub def: Type,
}

/// An expression (language.md 5).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Expr {
    /// Which form.
    pub kind: ExprKind,
    /// Where it is.
    pub span: Span,
}

/// Unary operators.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnOp {
    /// `-`
    Neg,
    /// `not`
    Not,
}

/// Binary operators (comparisons are in [`ExprKind::Compare`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `^`
    Pow,
    /// `..`
    Range,
    /// `in`
    In,
    /// `and`
    And,
    /// `or`
    Or,
}

/// Comparison operators.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CmpOp {
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
}

/// The sign of one tolerance term.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TolSign {
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `+-`
    PlusMinus,
}

/// One term of a tolerance bracket, such as `+0.018mm`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TolTerm {
    /// The sign.
    pub sign: TolSign,
    /// The magnitude.
    pub value: Expr,
}

/// A `match` arm.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MatchArm {
    /// The pattern.
    pub pat: Pattern,
    /// The result.
    pub body: Expr,
    /// Where it is.
    pub span: Span,
}

/// A `match` pattern.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Pattern {
    /// `_`
    Wildcard(Span),
    /// A variant, qualified or bare.
    Path(Path),
}

/// A block `{ let a = ...; expr }`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Block {
    /// The `let`s before the result.
    pub lets: Vec<Let>,
    /// The result expression.
    pub tail: Box<Expr>,
    /// Where it is, braces included.
    pub span: Span,
}

/// The expression forms.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ExprKind {
    /// A bare number, as written.
    Number(String),
    /// A quantity literal, as written.
    Quantity(Vec<Segment>),
    /// A string.
    Str(String),
    /// `true` or `false`.
    Bool(bool),
    /// A name or `::` path (including `self` and `origin`).
    Path(Path),
    /// `( expr )`
    Paren(Box<Expr>),
    /// `[a, b, c]`
    Array(Vec<Expr>),
    /// A unary operation.
    Unary(UnOp, Box<Expr>),
    /// A binary operation.
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// A chained comparison `a <= b <= c`.
    Compare {
        /// The first operand.
        first: Box<Expr>,
        /// Each further operator and operand.
        rest: Vec<(CmpOp, Expr)>,
    },
    /// `expr satisfies P`
    Satisfies(Box<Expr>, Box<Expr>),
    /// `expr as Type`
    As(Box<Expr>, Type),
    /// `expr.name`
    Field(Box<Expr>, Ident),
    /// `expr[index]`
    Index(Box<Expr>, Box<Expr>),
    /// `callee(args)` or `callee?(args)`.
    Call {
        /// The callee.
        callee: Box<Expr>,
        /// True for the `?(` form.
        optional: bool,
        /// Arguments.
        args: Vec<Arg>,
    },
    /// `expr [+0.1mm -0]`
    Tolerance(Box<Expr>, Vec<TolTerm>),
    /// `expr [position 0.2mm]`: a true-position zone diameter on a `Point2` value or array
    /// (geometry.md 3.1).
    Position(Box<Expr>, Box<Expr>),
    /// `expr H8`
    IsoClass(Box<Expr>, Ident),
    /// `if cond { } else { }`
    If {
        /// The condition.
        cond: Box<Expr>,
        /// The `then` block.
        then: Block,
        /// The `else` block or chained `if`.
        otherwise: Option<Box<Expr>>,
    },
    /// `match scrutinee { arms }`
    Match {
        /// The matched value.
        scrutinee: Box<Expr>,
        /// The arms.
        arms: Vec<MatchArm>,
    },
    /// A block expression.
    Block(Block),
}
