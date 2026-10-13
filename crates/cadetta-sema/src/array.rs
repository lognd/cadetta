//! Fixed arrays: `[a, b, c]` literals and `[T; N]` declarations (language.md 3.3, 3.8).
//!
//! An array holds elements of one kind: quantities of one dimension, points of
//! one arity, or values of one other variety. A declared `[T; N]` also fixes the
//! length, and a point element type (`Point2`, `Point3`) fixes the arity.
//! Datum typing (`Point2 on D`) is left to the geometry epic (geometry.md 3).

// frob:ticket 01M4ST7AFZGCN52434BQJRT5MM

use cadetta_syntax::ast::{Type, TypeKind};
use cadetta_syntax::span::Span;
use tracing::{debug, trace};

use super::{Evaluator, R, Scope, Val, diag};

/// True when `a` and `b` are values of one kind (same variety and dimension/arity).
fn same_kind(a: &Val, b: &Val) -> bool {
    match (a, b) {
        (Val::Quantity(x), Val::Quantity(y)) => x.dim() == y.dim(),
        (Val::Inexact(x), Val::Inexact(y)) => x.dim == y.dim,
        (Val::Quantity(x), Val::Inexact(y)) | (Val::Inexact(y), Val::Quantity(x)) => {
            *x.dim() == y.dim
        }
        (Val::Bool(_), Val::Bool(_)) | (Val::Text(_), Val::Text(_)) => true,
        (Val::Variant { enum_path: x, .. }, Val::Variant { enum_path: y, .. }) => x == y,
        (Val::Point(x), Val::Point(y)) => x.len() == y.len(),
        (Val::Array(x), Val::Array(y)) => match (x.first(), y.first()) {
            (Some(p), Some(q)) => same_kind(p, q),
            _ => true,
        },
        _ => false,
    }
}

/// The point arity a declared element type names, when it names one.
fn point_arity(ty: &Type) -> Option<usize> {
    let name = match &ty.kind {
        TypeKind::Path(p) => p.segments.last()?.name.as_str(),
        TypeKind::OnDatum { base, .. } => base.segments.last()?.name.as_str(),
        _ => return None,
    };
    match name {
        "Point2" => Some(2),
        "Point3" => Some(3),
        _ => None,
    }
}

impl<'r, 'p> Evaluator<'r, 'p> {
    /// Rejects an array literal whose elements are not all of one kind.
    pub(super) fn check_homogeneous(&self, items: &[Val], span: Span) -> R<()> {
        if let Some((first, rest)) = items.split_first()
            && let Some(bad) = rest.iter().position(|v| !same_kind(first, v))
        {
            debug!(index = bad + 1, "array: mixed kinds");
            return Err(diag(
                "EVAL",
                span,
                format!(
                    "array element {} is not of the same kind as the first element",
                    bad + 1
                ),
            ));
        }
        trace!(n = items.len(), "array: elements share one kind");
        Ok(())
    }

    /// Checks a value against its declared `[T; N]` type: length N, point arity of T.
    pub(super) fn check_declared(
        &mut self,
        sc: &mut Scope,
        ty: &Type,
        value: &Val,
        span: Span,
    ) -> R<()> {
        let TypeKind::Array { elem, len } = &ty.kind else {
            return Ok(());
        };
        let Val::Array(items) = value else {
            return Err(diag(
                "EVAL",
                span,
                "an array type `[T; N]` needs an array value",
            ));
        };
        let want = self.count_arg(sc, len, "array length")?;
        if u64::try_from(items.len()).ok() != Some(want) {
            return Err(diag(
                "EVAL",
                span,
                format!(
                    "the array has {} elements, its type says {want}",
                    items.len()
                ),
            ));
        }
        if let Some(n) = point_arity(elem)
            && let Some(bad) = items
                .iter()
                .position(|v| !matches!(v, Val::Point(c) if c.len() == n))
        {
            return Err(diag(
                "EVAL",
                span,
                format!("array element {bad} is not a Point{n}"),
            ));
        }
        debug!(n = want, "array: matches its declared type");
        Ok(())
    }
}
