// frob:ticket 01M5187T6Y9B3SMBS9DMPG9VFX
//! Lowering of `table` items into the IR (ir.md 3 `Table`; language.md 3.3).
//!
//! A table lowers to `PKG::[MOD::]Name` with its key column, value columns, rows (key plus one
//! cell per column, in source order) and its `@cite` as `citation`. Keys and cells come from the
//! evaluator ([`crate::eval::TableCells`]); a quantity cell keeps its authored source text. A table
//! the evaluator could not build, or one holding a cell that is neither a quantity nor an enum
//! variant, is skipped with a warning.

use cadetta_ir::entities::{Cell, Table, TableCol, TableRow};
use cadetta_ir::variant_name::ComponentPath;
use cadetta_syntax::ast;
use tracing::{debug, warn};

use super::{Cx, SourceText, quant_value, text_of};
use crate::cite::CiteIndex;
use crate::eval::Val;
use crate::resolve::CanonPath;

impl Cx<'_> {
    /// One table cell as an IR cell; `None` (logged by the caller) for a non-table value kind.
    fn lower_cell(&self, table: &CanonPath, v: &Val, authored: &str) -> Option<Cell> {
        match v {
            Val::Quantity(q) => Some(Cell::Value(Box::new(quant_value(
                self.input.evaluation,
                &self.owners,
                table,
                q,
                authored,
            )))),
            Val::Variant { variant, .. } => Some(Cell::Variant(variant.clone())),
            _ => None,
        }
    }

    /// Lowers table `t` of `module`; `None` when it did not evaluate or holds an unsupported cell.
    pub(super) fn lower_table(
        &self,
        canon: &CanonPath,
        src: Option<&SourceText<'_>>,
        idx: &CiteIndex,
        module: (&str, &[String]),
        t: &ast::Table,
    ) -> Option<Table> {
        let Some(cells) = self.input.evaluation.tables.get(canon) else {
            warn!(table = %canon, "lower: table has no evaluated cells; skipped");
            return None;
        };
        let key = t.key.as_ref()?;
        let col = |p: &ast::ParamDecl| TableCol {
            name: p.name.name.clone(),
            type_name: text_of(src, p.ty.span),
        };
        let mut rows = Vec::new();
        for (i, row) in t.rows.iter().enumerate() {
            let key_cell = cells
                .keys
                .get(i)
                .and_then(|v| self.lower_cell(canon, v, &text_of(src, row.key.span)));
            let vals = cells.rows.get(i).and_then(Option::as_ref);
            let mut out = Vec::new();
            let mut ok = key_cell.is_some();
            for (j, e) in row.values.iter().flatten().enumerate() {
                match vals
                    .and_then(|v| v.get(j))
                    .and_then(|v| self.lower_cell(canon, v, &text_of(src, e.span)))
                {
                    Some(c) => out.push(c),
                    None => ok = false,
                }
            }
            let (true, Some(key_cell)) = (ok, key_cell) else {
                warn!(table = %canon, row = i, "lower: table cell is not a quantity or variant; table skipped");
                return None;
            };
            rows.push(TableRow {
                key: key_cell,
                cells: out,
            });
        }
        let citation = t.citation.as_ref().and_then(|cc| {
            idx.lower_ref(module.0, module.1, cc)
                .map_err(
                    |e| warn!(table = %canon, code = e.code, "lower: table citation unresolved"),
                )
                .ok()
        });
        let path = ComponentPath::parse(&canon.to_string())
            .map_err(
                |e| warn!(table = %canon, error = %e, "lower: table path is not a component path"),
            )
            .ok()?;
        debug!(table = %canon, rows = rows.len(), cols = t.cols.len(), "lower: table");
        Some(Table {
            path,
            key: col(key),
            cols: t.cols.iter().map(col).collect(),
            rows,
            citation,
        })
    }
}
