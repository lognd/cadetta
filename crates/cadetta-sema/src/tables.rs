//! Tables and lookups: exact-match rows, `at_least` / `at_most` / `nearest`,
//! `.key`, key-only tables and `Table<K>` column checks (language.md 3.3, 3.6).
//!
//! The model is generic over the cell type so the evaluator can store its own
//! values; keys are compared exactly. The evaluator builds a [`TableData`]
//! (see [`TableData::from_ast`]) and calls the lookup methods, mapping a
//! [`LookupError`] to its lint code via [`LookupError::code`].

use std::cmp::Ordering;
use std::fmt;

use cadetta_syntax::ast;
use cadetta_units::{Dim, Magnitude, Sum};
use tracing::{debug, trace, warn};

/// The code for a lookup that matches no row (lints.md E007).
pub const E007: &str = "E007";
/// The code for a table shape or column mismatch (lints.md E008).
pub const E008: &str = "E008";
/// The code for an `ots` literal that matches no row (lints.md T001).
pub const T001: &str = "T001";

/// A table key: an enum variant or an exact quantity.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Key {
    /// An enum variant, by name.
    Variant(String),
    /// A quantity key: its dimension and exact SI magnitude.
    Quantity {
        /// The key's dimension.
        dim: Dim,
        /// The exact magnitude in SI base units.
        value: Magnitude,
    },
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Variant(v) => f.write_str(v),
            Self::Quantity { value, .. } => write!(f, "{value:?}"),
        }
    }
}

/// Why a table could not be built or read.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LookupError {
    /// No row matched the key (E007).
    Miss {
        /// The table.
        table: String,
        /// The key looked up.
        key: String,
    },
    /// The table has no such column (E008).
    NoSuchColumn {
        /// The table.
        table: String,
        /// The missing column.
        column: String,
    },
    /// A column was read from a key-only table (E008).
    KeyOnly {
        /// The table.
        table: String,
        /// The column read.
        column: String,
    },
    /// A range lookup on a table whose keys are not quantities, or with a
    /// differently-dimensioned key (E008).
    NotQuantityKeyed {
        /// The table.
        table: String,
    },
    /// A row does not match the table's `key` and `cols` (E008).
    Shape {
        /// The table.
        table: String,
        /// What is wrong.
        detail: String,
    },
}

impl LookupError {
    /// The lint code this error is reported under.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Miss { .. } => E007,
            _ => E008,
        }
    }
}

impl fmt::Display for LookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Miss { table, key } => write!(f, "no row of `{table}` has key {key}"),
            Self::NoSuchColumn { table, column } => {
                write!(f, "table `{table}` has no column `{column}`")
            }
            Self::KeyOnly { table, column } => write!(
                f,
                "table `{table}` is key-only; cannot read column `{column}`"
            ),
            Self::NotQuantityKeyed { table } => write!(
                f,
                "range lookup needs a quantity key of the table's dimension in `{table}`"
            ),
            Self::Shape { table, detail } => write!(f, "table `{table}`: {detail}"),
        }
    }
}

/// An `ots` literal citing a table that matches none of its rows (T001).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NotInTable {
    /// The table cited.
    pub table: String,
    /// The column the literal was compared against.
    pub column: String,
}

impl fmt::Display for NotInTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "literal matches no row of `{}` column `{}`",
            self.table, self.column
        )
    }
}

/// One row: its key and its cells (`None` for a key-only row).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Row<V> {
    /// The row key.
    pub key: Key,
    /// Cells in column order; `None` when the row has no `=>`.
    pub cells: Option<Vec<V>>,
}

/// A table with evaluated keys and cells.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TableData<V> {
    /// The table name.
    pub name: String,
    /// The key column's name.
    pub key_name: String,
    /// The value column names, empty for a key-only table.
    pub cols: Vec<String>,
    /// The rows in source order.
    pub rows: Vec<Row<V>>,
}

/// A matched row, read by `.key` or by column.
#[derive(Debug)]
pub struct Hit<'t, V> {
    table: &'t TableData<V>,
    row: &'t Row<V>,
}

impl<V> Clone for Hit<'_, V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<V> Copy for Hit<'_, V> {}

impl<'t, V> Hit<'t, V> {
    /// The matched key (`.key`), valid on every row including key-only ones.
    pub fn key(&self) -> &'t Key {
        &self.row.key
    }

    /// The cell in column `name`, rejecting key-only tables and unknown columns.
    pub fn column(&self, name: &str) -> Result<&'t V, LookupError> {
        let t = self.table;
        if t.cols.is_empty() {
            warn!(table = %t.name, column = name, "column read on key-only table");
            return Err(LookupError::KeyOnly {
                table: t.name.clone(),
                column: name.to_string(),
            });
        }
        let Some(i) = t.cols.iter().position(|c| c == name) else {
            warn!(table = %t.name, column = name, "no such column");
            return Err(LookupError::NoSuchColumn {
                table: t.name.clone(),
                column: name.to_string(),
            });
        };
        // from_ast guarantees every row of a table with columns has all cells
        Ok(&self
            .row
            .cells
            .as_ref()
            .expect("row of a keyed table has cells")[i])
    }
}

/// A quantity row with its key magnitude.
type KeyedRow<'t, V> = (&'t Magnitude, &'t Row<V>);

/// Order two exact magnitudes; `None` when an inexact sum is unordered.
fn cmp_mag(a: &Magnitude, b: &Magnitude) -> Option<Ordering> {
    match a.sub(b) {
        Sum::Exact(d) => {
            let n = d.rational().numer();
            let zero = n.clone() - n.clone();
            Some(n.cmp(&zero))
        }
        Sum::Inexact(f) => f.partial_cmp(&0.0),
    }
}

impl<V> TableData<V> {
    fn miss(&self, key: impl fmt::Display) -> LookupError {
        debug!(table = %self.name, key = %key, "lookup miss");
        LookupError::Miss {
            table: self.name.clone(),
            key: key.to_string(),
        }
    }

    fn hit<'t>(&'t self, row: &'t Row<V>) -> Hit<'t, V> {
        Hit { table: self, row }
    }

    /// `T[k]`: the row whose key equals `k` exactly, else a miss (E007).
    pub fn exact(&self, k: &Key) -> Result<Hit<'_, V>, LookupError> {
        match self.rows.iter().find(|r| &r.key == k) {
            Some(r) => {
                trace!(table = %self.name, key = %k, "exact hit");
                Ok(self.hit(r))
            }
            None => Err(self.miss(k)),
        }
    }

    /// Quantity rows paired with their magnitude, checking the key dimension.
    fn quantity_rows(&self, x: &Key) -> Result<(Magnitude, Vec<KeyedRow<'_, V>>), LookupError> {
        let bad = || LookupError::NotQuantityKeyed {
            table: self.name.clone(),
        };
        let Key::Quantity { dim, value } = x else {
            return Err(bad());
        };
        let mut rows = Vec::with_capacity(self.rows.len());
        for r in &self.rows {
            match &r.key {
                Key::Quantity { dim: d, value: v } if d == dim => rows.push((v, r)),
                _ => return Err(bad()),
            }
        }
        Ok((value.clone(), rows))
    }

    /// `T.at_least(x)`: the row with the smallest key `>= x`.
    pub fn at_least(&self, x: &Key) -> Result<Hit<'_, V>, LookupError> {
        let (x, rows) = self.quantity_rows(x)?;
        let mut best: Option<(&Magnitude, &Row<V>)> = None;
        for (v, r) in rows {
            if cmp_mag(v, &x).is_some_and(|o| o != Ordering::Less)
                && best.is_none_or(|(b, _)| cmp_mag(v, b) == Some(Ordering::Less))
            {
                best = Some((v, r));
            }
        }
        best.map(|(_, r)| self.hit(r))
            .ok_or_else(|| self.miss(format!("at_least {x:?}")))
    }

    /// `T.at_most(x)`: the row with the largest key `<= x`.
    pub fn at_most(&self, x: &Key) -> Result<Hit<'_, V>, LookupError> {
        let (x, rows) = self.quantity_rows(x)?;
        let mut best: Option<(&Magnitude, &Row<V>)> = None;
        for (v, r) in rows {
            if cmp_mag(v, &x).is_some_and(|o| o != Ordering::Greater)
                && best.is_none_or(|(b, _)| cmp_mag(v, b) == Some(Ordering::Greater))
            {
                best = Some((v, r));
            }
        }
        best.map(|(_, r)| self.hit(r))
            .ok_or_else(|| self.miss(format!("at_most {x:?}")))
    }

    /// `T.nearest(x)`: the closest key, ties to the larger.
    pub fn nearest(&self, x: &Key) -> Result<Hit<'_, V>, LookupError> {
        let (x, rows) = self.quantity_rows(x)?;
        let dist = |v: &Magnitude| -> Option<Magnitude> {
            let Sum::Exact(d) = v.sub(&x) else {
                return None;
            };
            Some(if cmp_mag(&d, &zero_of(&d)) == Some(Ordering::Less) {
                d.neg()
            } else {
                d
            })
        };
        let mut best: Option<(&Magnitude, Magnitude, &Row<V>)> = None;
        for (v, r) in rows {
            let Some(d) = dist(v) else { continue };
            let better = match &best {
                None => true,
                Some((bv, bd, _)) => match cmp_mag(&d, bd) {
                    Some(Ordering::Less) => true,
                    Some(Ordering::Equal) => cmp_mag(v, bv) == Some(Ordering::Greater),
                    _ => false,
                },
            };
            if better {
                best = Some((v, d, r));
            }
        }
        best.map(|(_, _, r)| self.hit(r))
            .ok_or_else(|| self.miss(format!("nearest {x:?}")))
    }

    /// `Table<K>` check: every column in `used` must exist, reported at the use
    /// site (E008). A key-only table has no columns, so any column is missing.
    pub fn require_columns(&self, used: &[&str]) -> Result<(), Vec<LookupError>> {
        let errs: Vec<LookupError> = used
            .iter()
            .filter(|c| !self.cols.iter().any(|x| x == **c))
            .map(|c| {
                warn!(table = %self.name, column = *c, "Table<K> argument lacks column");
                if self.cols.is_empty() {
                    LookupError::KeyOnly {
                        table: self.name.clone(),
                        column: (*c).to_string(),
                    }
                } else {
                    LookupError::NoSuchColumn {
                        table: self.name.clone(),
                        column: (*c).to_string(),
                    }
                }
            })
            .collect();
        if errs.is_empty() { Ok(()) } else { Err(errs) }
    }

    /// T001: does any row's `column` cell satisfy `matches`? Records the fact if none does.
    pub fn check_ots_literal(
        &self,
        column: &str,
        matches: impl Fn(&V) -> bool,
    ) -> Result<(), NotInTable> {
        let Some(i) = self.cols.iter().position(|c| c == column) else {
            return Ok(()); // an unknown column is E008 at the read, not T001
        };
        let found = self
            .rows
            .iter()
            .filter_map(|r| r.cells.as_ref())
            .any(|c| matches(&c[i]));
        if found {
            Ok(())
        } else {
            debug!(table = %self.name, column, "T001: literal matches no row");
            Err(NotInTable {
                table: self.name.clone(),
                column: column.to_string(),
            })
        }
    }

    /// Build from the AST, evaluating keys and cells with the caller's closures.
    ///
    /// Row-shape problems (arity, `=>` presence) are E008.
    pub fn from_ast<E>(
        t: &ast::Table,
        mut key: impl FnMut(&ast::Expr) -> Result<Key, E>,
        mut cell: impl FnMut(&ast::Expr) -> Result<V, E>,
    ) -> Result<Result<Self, LookupError>, E> {
        let name = t.name.name.clone();
        let shape = |detail: String| LookupError::Shape {
            table: name.clone(),
            detail,
        };
        let Some(kd) = &t.key else {
            return Ok(Err(shape("missing `key` declaration".into())));
        };
        let cols: Vec<String> = t.cols.iter().map(|c| c.name.name.clone()).collect();
        let mut rows = Vec::with_capacity(t.rows.len());
        for r in &t.rows {
            let k = key(&r.key)?;
            let cells = match (&r.values, cols.is_empty()) {
                (None, true) => None,
                (Some(v), false) if v.len() == cols.len() => {
                    let mut out = Vec::with_capacity(v.len());
                    for e in v {
                        out.push(cell(e)?);
                    }
                    Some(out)
                }
                (Some(v), false) => {
                    return Ok(Err(shape(format!(
                        "row {k} has {} values for {} columns",
                        v.len(),
                        cols.len()
                    ))));
                }
                (None, false) => {
                    return Ok(Err(shape(format!("row {k} has no `=>` values"))));
                }
                (Some(_), true) => {
                    return Ok(Err(shape(format!("row {k} has values but no `cols`"))));
                }
            };
            rows.push(Row { key: k, cells });
        }
        debug!(table = %name, rows = rows.len(), cols = cols.len(), "table built");
        Ok(Ok(Self {
            name: name.clone(),
            key_name: kd.name.name.clone(),
            cols,
            rows,
        }))
    }
}

/// A zero magnitude with no shared state (so comparisons need no extra import).
fn zero_of(m: &Magnitude) -> Magnitude {
    match m.sub(m) {
        Sum::Exact(z) => z,
        Sum::Inexact(_) => unreachable!("x - x has equal pi exponents"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_units::Rational;

    fn len(mm: i64) -> Key {
        Key::Quantity {
            dim: Dim::default(),
            value: Magnitude::new(Rational::from_int(mm), 0),
        }
    }
    fn var(s: &str) -> Key {
        Key::Variant(s.into())
    }
    fn keyonly() -> TableData<i64> {
        TableData {
            name: "Iso4762Len".into(),
            key_name: "length".into(),
            cols: vec![],
            rows: [3, 4, 5, 8]
                .iter()
                .map(|n| Row {
                    key: len(*n),
                    cells: None,
                })
                .collect(),
        }
    }
    fn threads() -> TableData<i64> {
        TableData {
            name: "Iso273".into(),
            key_name: "thread".into(),
            cols: vec!["fine".into(), "medium".into()],
            rows: vec![
                Row {
                    key: var("M5"),
                    cells: Some(vec![53, 55]),
                },
                Row {
                    key: var("M6"),
                    cells: Some(vec![64, 66]),
                },
            ],
        }
    }

    #[test]
    fn exact_match_reads_column() {
        let t = threads();
        assert_eq!(t.exact(&var("M6")).unwrap().column("medium"), Ok(&66));
    }

    #[test]
    fn range_lookups_and_ties() {
        let t = keyonly();
        assert_eq!(t.at_least(&len(4)).unwrap().key(), &len(4));
        assert_eq!(t.at_least(&len(6)).unwrap().key(), &len(8));
        assert_eq!(t.at_most(&len(7)).unwrap().key(), &len(5));
        assert_eq!(t.nearest(&len(6)).unwrap().key(), &len(5)); // 1 vs 2
        assert_eq!(t.nearest(&len(7)).unwrap().key(), &len(8));
        // exact tie between 4 and 8 goes to the larger
        let mut t2 = keyonly();
        t2.rows.retain(|r| r.key == len(4) || r.key == len(8));
        assert_eq!(t2.nearest(&len(6)).unwrap().key(), &len(8));
    }

    #[test]
    fn exact_key_on_any_row() {
        assert_eq!(threads().exact(&var("M5")).unwrap().key(), &var("M5"));
    }

    #[test]
    fn miss_is_e007() {
        let e = threads().exact(&var("M9")).unwrap_err();
        assert_eq!(e.code(), "E007");
        let e = keyonly().at_least(&len(100)).unwrap_err();
        assert_eq!(e.code(), "E007");
        let e = keyonly().at_most(&len(1)).unwrap_err();
        assert_eq!(e.code(), "E007");
    }

    #[test]
    fn key_only_rejects_columns() {
        let t = keyonly();
        let h = t.at_least(&len(4)).unwrap();
        assert_eq!(h.key(), &len(4));
        let e = h.column("medium").unwrap_err();
        assert!(matches!(e, LookupError::KeyOnly { .. }));
        assert_eq!(e.code(), "E008");
    }

    #[test]
    fn table_param_missing_column_is_e008() {
        let t = threads();
        assert!(t.require_columns(&["medium"]).is_ok());
        let errs = t.require_columns(&["medium", "coarse"]).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code(), "E008");
        assert!(errs[0].to_string().contains("Iso273"));
    }

    #[test]
    fn ots_literal_not_in_table_is_t001() {
        let t = threads();
        assert!(t.check_ots_literal("medium", |v| *v == 66).is_ok());
        let f = t.check_ots_literal("medium", |v| *v == 99).unwrap_err();
        assert_eq!(f.table, "Iso273");
    }
}
