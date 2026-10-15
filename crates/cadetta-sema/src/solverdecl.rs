//! frob:ticket 01M4V2K8RG8K1P774YVYBFVYDC
//! frob:ticket 01M4V3F2A8PNNRXND41CXE9G4M
//! Check-time validation of `solver` declarations (solvers.md 1): S007, Q001 and E003.

use cadetta_solve::decl::{SolverDeclError, lower_solver};
use cadetta_syntax::ast::{File, FileEntry, ItemKind, MemberKind, Solver};
use tracing::debug;

/// Every `solver` declaration in a file, top-level and component members.
fn solvers(file: &File) -> Vec<&Solver> {
    let mut out = Vec::new();
    for e in &file.entries {
        let FileEntry::Item(i) = e else { continue };
        match &i.kind {
            ItemKind::Solver(s) => out.push(s),
            ItemKind::Component(c) => out.extend(c.members.iter().filter_map(|m| match &m.kind {
                MemberKind::Solver(s) => Some(s),
                _ => None,
            })),
            _ => {}
        }
    }
    out
}

/// Declaration diagnostics (S007, Q001, E003) of every solver in `file`.
pub fn check_solver_decls(file: &File) -> Vec<SolverDeclError> {
    let mut out = Vec::new();
    for s in solvers(file) {
        if let Err(errs) = lower_solver(s, &s.name.name) {
            debug!(solver = %s.name.name, errors = errs.len(), "solver declaration rejected");
            out.extend(errs);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadetta_syntax::parser::parse;

    fn codes(src: &str) -> Vec<&'static str> {
        let p = parse(src);
        assert!(p.diags.is_empty(), "{:?}", p.diags);
        check_solver_decls(&p.file).iter().map(|e| e.code).collect()
    }

    // frob:tests crates/cadetta-sema/src/solverdecl.rs::check_solver_decls kind=unit
    #[test]
    fn untyped_out_is_s007_not_a_syntax_error() {
        let src = "solver s {\n  script \"s.py\"\n  in a = 1mm\n  out wall\n}\n";
        assert_eq!(codes(src), ["S007"]);
    }

    // frob:tests crates/cadetta-sema/src/solverdecl.rs::check_solver_decls kind=unit
    #[test]
    fn bad_in_forms_and_legal_forms() {
        let bad = |e: &str| {
            format!("solver s {{\n  script \"s.py\"\n  in a = {e}\n  out o: Length\n}}\n")
        };
        assert_eq!(codes(&bad("\"x\"")), ["S007"]);
        assert_eq!(codes(&bad("3")), ["Q001"]);
        assert!(codes(&bad("2 * x.y")).is_empty());
    }
}
