//! Bridge from `tpt_rustir_core::Term` to the e-graph `Expr` language.
//!
//! The MIR bridge (`tpt-rustir-mir`) lowers Rust arithmetic into core terms
//! built from the stdlib constants defined in `tpt_rustir_mir::stdlib`
//! (`add`, `mul`, `le`). This module translates the subset of core terms that
//! arises from such obligations — Nat/Bool values, de Bruijn variables
//! (treated as free symbols), and applications of the stdlib heads — into
//! `Expr` trees so the tactic engine (and, behind the `smt` feature, Z3) can
//! discharge them.
//!
//! Terms containing anything outside this subset (Pi, Sigma, pairs, inductive
//! eliminators, unknown application heads, …) fail to translate with `None`;
//! callers should then mark the obligation as not automatically checkable.

use std::str::FromStr;

use egg::RecExpr;

use crate::lang::Expr;
use tpt_rustir_core::term::{Term, TermKind};

/// Symbolic name for the free de Bruijn variable with index `n`. Indices are
/// counted from the innermost binder, matching the kernel's convention.
pub fn var_symbol(n: usize) -> String {
    format!("v{n}")
}

/// Sanitize a constant name into a legal `egg` symbol (alphanumeric plus `_`).
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

/// If `t` is the application spine `Const(name) a b …` with exactly `k`
/// arguments, return them in application order, else `None`.
fn head_app<'a>(t: &'a Term, name: &str, k: usize) -> Option<Vec<&'a Term>> {
    let mut args = Vec::with_capacity(k);
    let mut cur = t;
    loop {
        match &**cur {
            TermKind::App(f, a) => {
                args.push(a);
                cur = f;
            }
            TermKind::Const(h) if h == name => break,
            _ => return None,
        }
    }
    if args.len() != k {
        return None;
    }
    args.reverse();
    Some(args)
}

fn atom(out: &mut String, s: &str) {
    if !out.is_empty() {
        out.push(' ');
    }
    out.push_str(s);
}

fn build(t: &Term, out: &mut String) -> Option<()> {
    match &**t {
        TermKind::Zero => atom(out, "zero"),
        TermKind::True => atom(out, "true"),
        TermKind::False => atom(out, "false"),
        TermKind::Succ(n) => {
            atom(out, "(succ");
            build(n, out)?;
            out.push(')');
        }
        TermKind::Var(n) => atom(out, &var_symbol(*n)),
        TermKind::Const(name) => atom(out, &sanitize(name)),
        TermKind::App(_, _) => {
            let (head, args): (String, Vec<&Term>) = if let Some(a) = head_app(t, "add", 2) {
                ("+".to_string(), a)
            } else if let Some(a) = head_app(t, "mul", 2) {
                ("*".to_string(), a)
            } else if let Some(a) = head_app(t, "le", 2) {
                ("le".to_string(), a)
            } else {
                return None;
            };
            atom(out, &format!("({head}"));
            for a in &args {
                build(a, out)?;
            }
            out.push(')');
        }
        // Types and anything else are outside the obligation subset.
        _ => return None,
    }
    Some(())
}

/// Translate a core term into an `Expr` tree. Returns `None` when the term is
/// outside the supported arithmetic/propositional subset.
pub fn term_to_expr(t: &Term) -> Option<RecExpr<Expr>> {
    let mut s = String::new();
    build(t, &mut s)?;
    RecExpr::<Expr>::from_str(&s).ok()
}

/// Translate a core term into an `Expr` tree, reporting the intermediate
/// s-expression on failure (useful for diagnostics).
pub fn term_to_expr_lossy(t: &Term) -> Result<RecExpr<Expr>, String> {
    let mut s = String::new();
    match build(t, &mut s) {
        Some(()) => RecExpr::<Expr>::from_str(&s).map_err(|e| format!("{e}: {s}")),
        None => Err(format!(
            "term outside the arithmetic/propositional obligation subset: {t:?}"
        )),
    }
}

/// Re-export for callers that want to name variables consistently.
pub use egg::Symbol as EggSymbol;

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_rustir_core::term;

    #[test]
    fn numerals_and_ops_translate() {
        // (add (succ zero) (succ (succ zero)))
        let t = term::app(
            term::app(
                term::const_("add"),
                term::succ(term::zero()),
            ),
            term::succ(term::succ(term::zero())),
        );
        let e = term_to_expr(&t).expect("arithmetic term should translate");
        assert_eq!(e.to_string(), "(+ (succ zero) (succ (succ zero)))");
    }

    #[test]
    fn vars_become_symbols() {
        // (le v1 v0):  de Bruijn Var(1), Var(0)
        let t = term::app(
            term::app(term::const_("le"), term::var(1)),
            term::var(0),
        );
        let e = term_to_expr(&t).expect("le term should translate");
        assert!(e.to_string().contains("(le v1 v0)"));
    }

    #[test]
    fn unsupported_terms_are_rejected() {
        assert!(term_to_expr(&term::universe(0)).is_none());
        assert!(term_to_expr(&term::nat()).is_none());
        let bad = term::app(term::const_("frobnicate"), term::zero());
        assert!(term_to_expr(&bad).is_none());
    }
}

