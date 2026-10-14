//! Enum values and exhaustive `match` (language.md 3.8).
//!
//! A `match` scrutinee evaluates to a [`Val::Variant`]. Each arm is a bare
//! variant of the scrutinee's enum, a qualified `Enum::Variant`, or `_`. A
//! match without `_` must name every variant: otherwise E009 lists the
//! missing ones. The first matching arm is the only arm evaluated.

// frob:ticket 01M4ST7ASWYQV13AAHHCXVR570

use std::collections::BTreeSet;

use cadetta_syntax::ast::{Expr, MatchArm, Pattern};
use cadetta_syntax::span::Span;
use tracing::{debug, trace};

use super::{EvalDiagnostic, Evaluator, Fail, R, Scope, Val, diag};
use crate::deps::Severity;
use crate::resolve::{CanonPath, ModuleKey, Resolved};

/// What one arm matches.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum ArmPat {
    /// `_`: every variant.
    Wild,
    /// One named variant.
    Variant(String),
}

/// The variants of `all` that no arm names, in declaration order.
pub fn missing_variants(all: &[String], covered: &BTreeSet<String>) -> Vec<String> {
    all.iter()
        .filter(|v| !covered.contains(*v))
        .cloned()
        .collect()
}

/// The E009 message naming the missing variants of `enum_name`.
pub fn non_exhaustive_message(enum_name: &str, missing: &[String]) -> String {
    let list = missing
        .iter()
        .map(|v| format!("`{enum_name}::{v}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("`match` is not exhaustive: missing {list}")
}

impl Evaluator<'_, '_> {
    /// Evaluates `match scrutinee { arms }`: E009 when a variant is unhandled.
    pub(super) fn eval_match(
        &mut self,
        sc: &mut Scope,
        scrutinee: &Expr,
        arms: &[MatchArm],
        span: Span,
    ) -> R<Val> {
        let Val::Variant { enum_path, variant } = self.eval(sc, scrutinee)? else {
            return Err(diag(
                "EVAL",
                scrutinee.span,
                "the value matched must be an enum variant",
            ));
        };
        let all: Vec<String> = self
            .resolver
            .enum_variants(&enum_path)
            .map(<[String]>::to_vec)
            .unwrap_or_default();
        let mut pats = Vec::with_capacity(arms.len());
        for a in arms {
            pats.push(self.arm_pattern(sc, &enum_path, &all, &a.pat)?);
        }
        let covered: BTreeSet<String> = pats
            .iter()
            .filter_map(|p| match p {
                ArmPat::Variant(v) => Some(v.clone()),
                ArmPat::Wild => None,
            })
            .collect();
        if !pats.contains(&ArmPat::Wild) {
            let missing = missing_variants(&all, &covered);
            if !missing.is_empty() {
                debug!(?missing, "match: not exhaustive");
                return Err(Fail::Diag(EvalDiagnostic {
                    module: Some(ModuleKey::of_ctx(&sc.ctx)),
                    code: "E009",
                    severity: Severity::Error,
                    message: non_exhaustive_message(&enum_path.item, &missing),
                    span,
                }));
            }
        }
        let chosen = pats.iter().position(|p| match p {
            ArmPat::Wild => true,
            ArmPat::Variant(v) => *v == variant,
        });
        let Some(i) = chosen else {
            return Err(diag("EVAL", span, "no `match` arm matches the value"));
        };
        trace!(arm = i, %variant, "match: arm selected");
        self.eval(sc, &arms[i].body)
    }

    fn arm_pattern(
        &self,
        sc: &Scope,
        enum_path: &CanonPath,
        all: &[String],
        pat: &Pattern,
    ) -> R<ArmPat> {
        let Pattern::Path(p) = pat else {
            return Ok(ArmPat::Wild);
        };
        if let [only] = p.segments.as_slice() {
            return if all.contains(&only.name) {
                Ok(ArmPat::Variant(only.name.clone()))
            } else {
                Err(diag(
                    "E002",
                    p.span,
                    format!("enum `{}` has no variant `{}`", enum_path.item, only.name),
                ))
            };
        }
        match self.resolver.resolve_path(&sc.ctx, p, None) {
            Ok(Resolved::Variant {
                enum_path: ep,
                variant,
            }) if ep == *enum_path => Ok(ArmPat::Variant(variant)),
            Ok(_) => Err(diag(
                "E006",
                p.span,
                format!("pattern is not a variant of `{}`", enum_path.item),
            )),
            Err(re) => Err(diag(re.code, re.span, re.message)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    // frob:tests missing_variants_lists_the_uncovered
    #[test]
    fn missing_variants_lists_the_uncovered() {
        let all = names(&["A", "B", "C"]);
        let covered: BTreeSet<String> = names(&["B"]).into_iter().collect();
        assert_eq!(missing_variants(&all, &covered), names(&["A", "C"]));
        assert_eq!(
            non_exhaustive_message("E", &names(&["A", "C"])),
            "`match` is not exhaustive: missing `E::A`, `E::C`"
        );
    }
}
