//! AST → Core lowering ("elaboration"): resolves names to de Bruijn indices
//! and recognizes builtin heads (`zero`, `succ`, `Nat`, `natrec`, ...).

use std::fmt;

use tpt_rustir_core::term::{self, Term};

use crate::ast::{Expr, ExprKind};

#[derive(Debug, Clone)]
pub enum LowerError {
    ArityMismatch {
        name: &'static str,
        expected: usize,
        found: usize,
    },
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LowerError::ArityMismatch {
                name,
                expected,
                found,
            } => write!(f, "`{name}` expects {expected} argument(s), found {found}"),
        }
    }
}

impl std::error::Error for LowerError {}

/// Lexical scope: names in binding order, nearest-bound last.
#[derive(Default, Clone)]
pub struct Scope(Vec<String>);

impl Scope {
    pub fn new() -> Self {
        Scope(Vec::new())
    }

    fn resolve(&self, name: &str) -> Option<usize> {
        self.0.iter().rev().position(|n| n == name)
    }

    fn push(&self, name: &str) -> Scope {
        let mut s = self.clone();
        s.0.push(name.to_string());
        s
    }
}

/// Flatten a left-nested application spine into (head, args).
fn spine(e: &Expr) -> (&Expr, Vec<&Expr>) {
    let mut args = Vec::new();
    let mut cur = e;
    while let ExprKind::App(f, a) = &cur.kind {
        args.push(a.as_ref());
        cur = f;
    }
    args.reverse();
    (cur, args)
}

fn builtin_nullary(name: &str) -> Option<Term> {
    match name {
        "Nat" => Some(term::nat()),
        "Bool" => Some(term::bool_ty()),
        "zero" => Some(term::zero()),
        "true" => Some(term::true_()),
        "false" => Some(term::false_()),
        "Type" => Some(term::universe(0)),
        _ => {
            let rest = name.strip_prefix("Type")?;
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                Some(term::universe(rest.parse().unwrap()))
            } else {
                None
            }
        }
    }
}

pub fn lower(scope: &Scope, e: &Expr) -> Result<Term, LowerError> {
    match &e.kind {
        ExprKind::Var(name) => {
            if let Some(t) = builtin_nullary(name) {
                return Ok(t);
            }
            match scope.resolve(name) {
                Some(idx) => Ok(term::var(idx)),
                None => Ok(term::const_(name.clone())),
            }
        }
        ExprKind::NatLit(n) => {
            let mut t = term::zero();
            for _ in 0..*n {
                t = term::succ(t);
            }
            Ok(t)
        }
        ExprKind::Pi(name, dom, cod) => {
            let dom_t = lower(scope, dom)?;
            let cod_t = lower(&scope.push(name), cod)?;
            Ok(term::pi(dom_t, cod_t))
        }
        ExprKind::Sigma(name, fst_ty, snd_ty) => {
            let fst_t = lower(scope, fst_ty)?;
            let snd_t = lower(&scope.push(name), snd_ty)?;
            Ok(term::sigma(fst_t, snd_t))
        }
        ExprKind::Lambda(name, dom, body) => {
            let dom_t = lower(scope, dom)?;
            let body_t = lower(&scope.push(name), body)?;
            Ok(term::lambda(dom_t, body_t))
        }
        ExprKind::Pair(a, b) => {
            // Bare pairs need a Sigma annotation; only valid inside `Ann` or `check`.
            // As a standalone term we can't infer the type, so annotate with a
            // placeholder that will fail unless the caller supplies one via Ann.
            let a_t = lower(scope, a)?;
            let b_t = lower(scope, b)?;
            Ok(term::pair(
                a_t.clone(),
                b_t,
                term::sigma(term::nat(), term::nat()),
            ))
        }
        ExprKind::Let(name, ann_ty, val, body) => {
            let ty_t = lower(scope, ann_ty)?;
            let val_t = lower(scope, val)?;
            let body_t = lower(&scope.push(name), body)?;
            Ok(term::let_(ty_t, val_t, body_t))
        }
        ExprKind::App(_, _) => {
            let (head, args) = spine(e);
            if let ExprKind::Var(name) = &head.kind {
                match name.as_str() {
                    "succ" => return apply_unary(scope, "succ", &args, term::succ),
                    "fst" => return apply_unary(scope, "fst", &args, term::fst),
                    "snd" => return apply_unary(scope, "snd", &args, term::snd),
                    "natrec" => return apply_quaternary(scope, "natrec", &args, term::nat_rec),
                    "boolrec" => return apply_quaternary(scope, "boolrec", &args, term::bool_rec),
                    _ => {}
                }
            }
            let head_t = lower(scope, head)?;
            args.into_iter()
                .try_fold(head_t, |f, a| Ok(term::app(f, lower(scope, a)?)))
        }
        ExprKind::Ann(inner, ty) => {
            let ty_t = lower(scope, ty)?;
            // Pairs need their Sigma annotation threaded through from the ascription.
            if let ExprKind::Pair(a, b) = &inner.kind {
                let a_t = lower(scope, a)?;
                let b_t = lower(scope, b)?;
                return Ok(term::pair(a_t, b_t, ty_t));
            }
            lower(scope, inner)
        }
    }
}

fn apply_unary(
    scope: &Scope,
    name: &'static str,
    args: &[&Expr],
    f: impl Fn(Term) -> Term,
) -> Result<Term, LowerError> {
    if args.len() != 1 {
        return Err(LowerError::ArityMismatch {
            name,
            expected: 1,
            found: args.len(),
        });
    }
    Ok(f(lower(scope, args[0])?))
}

fn apply_quaternary(
    scope: &Scope,
    name: &'static str,
    args: &[&Expr],
    f: impl Fn(Term, Term, Term, Term) -> Term,
) -> Result<Term, LowerError> {
    if args.len() != 4 {
        return Err(LowerError::ArityMismatch {
            name,
            expected: 4,
            found: args.len(),
        });
    }
    Ok(f(
        lower(scope, args[0])?,
        lower(scope, args[1])?,
        lower(scope, args[2])?,
        lower(scope, args[3])?,
    ))
}
