//! Table values in the evaluator: `T`, `T[key]`, `T.at_least(x).key`, `row.column`
//! (language.md 3.3, 3.6). Tables are built on first use from their AST with the
//! evaluator's own cell values, then read through [`crate::tables`].

// frob:ticket 01M4TRBJ7D2JTGX6XX94BJ1ZGM

use std::rc::Rc;

use cadetta_syntax::ast::{self, Expr, ExprKind, Ident};
use cadetta_syntax::span::Span;
use tracing::{debug, trace};

use super::{Evaluator, Fail, R, Scope, Val, diag, strip};
use crate::resolve::{CanonPath, Ctx, ModuleKey, Resolved};
use crate::tables::{Key, LookupError, TableData};

/// The range-lookup methods of a table (language.md 3.6).
pub(super) const LOOKUPS: [&str; 3] = ["at_least", "at_most", "nearest"];

/// A table with evaluated keys (as values) and cells.
#[derive(Debug)]
pub(super) struct TableVals {
    /// Keys and cells, with the lookup methods.
    data: TableData<Val>,
    /// Each row's key as a value, in row order (so `.key` keeps its enum path).
    keys: Vec<Val>,
}

fn lookup_diag(e: &LookupError, span: Span) -> Fail {
    diag(e.code(), span, e.to_string())
}

impl<'r, 'p> Evaluator<'r, 'p> {
    /// The table at `path`, built and cached on first use.
    pub(super) fn table_vals(&mut self, path: &CanonPath, span: Span) -> R<Rc<TableVals>> {
        if let Some(t) = self.table_cache.get(path) {
            return Ok(Rc::clone(t));
        }
        let Some(ast) = self.tables.get(path).copied() else {
            return Err(diag("EVAL", span, format!("table `{path}` is not known")));
        };
        let sc = Scope {
            ctx: Ctx {
                package: path.package.clone(),
                module: path.modules.clone(),
                component: None,
                locals: Vec::new(),
            },
            locals: Vec::new(),
        };
        let key_ty = ast.key.as_ref().map(|k| &k.ty);
        let keys = std::cell::RefCell::new(Vec::new());
        let state = std::cell::RefCell::new((&mut *self, sc));
        let built = TableData::from_ast(
            ast,
            |e| {
                let (ev, sc) = &mut *state.borrow_mut();
                let v = ev.eval_typed(sc, e, key_ty)?;
                let k = val_key(&v, e.span)?;
                keys.borrow_mut().push(v);
                Ok(k)
            },
            |e| {
                let (ev, sc) = &mut *state.borrow_mut();
                ev.eval(sc, e)
            },
        )
        .map_err(|e| e.in_module(ModuleKey::of_path(path)))?;
        let keys = keys.into_inner();
        let data = built.map_err(|e| lookup_diag(&e, span))?;
        debug!(%path, rows = data.rows.len(), "evaluator: table built");
        let t = Rc::new(TableVals { data, keys });
        self.table_cache.insert(path.clone(), Rc::clone(&t));
        Ok(t)
    }

    /// Evaluates a key or cell, resolving a bare enum variant against `ty`.
    fn eval_typed(&mut self, sc: &mut Scope, e: &Expr, ty: Option<&ast::Type>) -> R<Val> {
        if let (ExprKind::Path(p), Some(ty)) = (&strip(e).kind, ty)
            && p.segments.len() == 1
            && let Ok(Resolved::Variant { enum_path, variant }) =
                self.resolver.resolve_ref(&sc.ctx, strip(e), Some(ty))
        {
            trace!(%variant, "table: bare variant resolved by the declared type");
            return Ok(Val::Variant { enum_path, variant });
        }
        self.eval(sc, e)
    }

    /// `T[key]`: the row whose key equals `key` exactly (E007 on a miss).
    pub(super) fn table_index(&mut self, table: &CanonPath, key: Val, span: Span) -> R<Val> {
        let t = self.table_vals(table, span)?;
        let k = val_key(&key, span)?;
        let hit = t.data.exact(&k).map_err(|e| lookup_diag(&e, span))?;
        let row = t.data.rows.iter().position(|r| &r.key == hit.key());
        Ok(Val::Row {
            table: table.clone(),
            row: row.expect("a hit is a row of its table"),
        })
    }

    /// `T.at_least(x)`, `.at_most(x)` or `.nearest(x)`: the matching row.
    pub(super) fn table_lookup(
        &mut self,
        table: &CanonPath,
        method: &str,
        key: &Val,
        span: Span,
    ) -> R<Val> {
        let t = self.table_vals(table, span)?;
        let k = val_key(key, span)?;
        let hit = match method {
            "at_least" => t.data.at_least(&k),
            "at_most" => t.data.at_most(&k),
            _ => t.data.nearest(&k),
        }
        .map_err(|e| lookup_diag(&e, span))?;
        let row = t.data.rows.iter().position(|r| &r.key == hit.key());
        Ok(Val::Row {
            table: table.clone(),
            row: row.expect("a hit is a row of its table"),
        })
    }

    /// `row.key` or `row.column` on a looked-up row.
    pub(super) fn row_field(
        &mut self,
        table: &CanonPath,
        row: usize,
        name: &Ident,
        span: Span,
    ) -> R<Val> {
        let t = self.table_vals(table, span)?;
        if name.name == "key" {
            return Ok(t.keys[row].clone());
        }
        let r = &t.data.rows[row];
        let hit = t
            .data
            .exact(&r.key)
            .expect("a row's own key is in its table");
        hit.column(&name.name)
            .cloned()
            .map_err(|e| lookup_diag(&e, name.span))
    }
}

/// The table key a value stands for: a variant or an exact quantity.
fn val_key(v: &Val, span: Span) -> R<Key> {
    match v {
        Val::Variant { variant, .. } => Ok(Key::Variant(variant.clone())),
        Val::Quantity(q) => Ok(Key::Quantity {
            dim: q.dim().clone(),
            value: q.nominal.clone(),
        }),
        _ => Err(diag(
            "EVAL",
            span,
            "a table key is an enum variant or a quantity",
        )),
    }
}
