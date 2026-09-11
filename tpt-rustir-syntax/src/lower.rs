//! AST → Core lowering ("elaboration"): resolves names to de Bruijn indices
//! and recognizes builtin heads (`zero`, `succ`, `Nat`, `natrec`, ...).

use std::collections::HashMap;
use std::fmt;

use tpt_rustir_core::env::{ConstructorDef, GlobalEnv, InductiveDef};
use tpt_rustir_core::term::{self, Term};

use crate::ast::{Expr, ExprKind};

#[derive(Debug, Clone)]
pub enum LowerError {
    ArityMismatch {
        name: &'static str,
        expected: usize,
        found: usize,
    },
    DuplicateInductive(String),
    DuplicateConstructor(String, String),
    MissingConstructor(String),
    InvalidConstructorType,
}

impl fmt::Display for LowerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LowerError::ArityMismatch {
                name,
                expected,
                found,
            } => write!(f, "`{name}` expects {expected} argument(s), found {found}"),
            LowerError::DuplicateInductive(name) => write!(f, "duplicate inductive type `{name}`"),
            LowerError::DuplicateConstructor(ind, con) => {
                write!(f, "duplicate constructor `{con}` for inductive `{ind}`")
            }
            LowerError::MissingConstructor(name) => write!(f, "missing constructor `{name}`"),
            LowerError::InvalidConstructorType => write!(
                f,
                "constructor type must be a Pi type ending in the inductive type"
            ),
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

/// Result of lowering that may register new inductives in the environment.
pub struct LowerResult {
    pub term: Term,
    pub env: GlobalEnv,
}

impl LowerResult {
    pub fn new(term: Term) -> Self {
        Self {
            term,
            env: GlobalEnv::new(),
        }
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

/// Lower an expression and register any inductive declarations in the environment.
pub fn lower(scope: &Scope, env: &mut GlobalEnv, e: &Expr) -> Result<Term, LowerError> {
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
            let dom_t = lower(scope, env, dom)?;
            let cod_t = lower(&scope.push(name), env, cod)?;
            Ok(term::pi(dom_t, cod_t))
        }
        ExprKind::Sigma(name, fst_ty, snd_ty) => {
            let fst_t = lower(scope, env, fst_ty)?;
            let snd_t = lower(&scope.push(name), env, snd_ty)?;
            Ok(term::sigma(fst_t, snd_t))
        }
        ExprKind::Lambda(name, dom, body) => {
            let dom_t = lower(scope, env, dom)?;
            let body_t = lower(&scope.push(name), env, body)?;
            Ok(term::lambda(dom_t, body_t))
        }
        ExprKind::Pair(a, b) => {
            let a_t = lower(scope, env, a)?;
            let b_t = lower(scope, env, b)?;
            Ok(term::pair(
                a_t.clone(),
                b_t,
                term::sigma(term::nat(), term::nat()),
            ))
        }
        ExprKind::Let(name, ann_ty, val, body) => {
            let ty_t = lower(scope, env, ann_ty)?;
            let val_t = lower(scope, env, val)?;
            let body_t = lower(&scope.push(name), env, body)?;
            Ok(term::let_(ty_t, val_t, body_t))
        }
        ExprKind::App(_, _) => {
            let (head, args) = spine(e);
            if let ExprKind::Var(name) = &head.kind {
                match name.as_str() {
                    "succ" => return apply_unary(scope, env, "succ", &args, term::succ),
                    "fst" => return apply_unary(scope, env, "fst", &args, term::fst),
                    "snd" => return apply_unary(scope, env, "snd", &args, term::snd),
                    "natrec" => {
                        return apply_quaternary(scope, env, "natrec", &args, term::nat_rec)
                    }
                    "boolrec" => {
                        return apply_quaternary(scope, env, "boolrec", &args, term::bool_rec)
                    }
                    _ => {}
                }
            }
            let head_t = lower(scope, env, head)?;
            args.into_iter()
                .try_fold(head_t, |f, a| Ok(term::app(f, lower(scope, env, a)?)))
        }
        ExprKind::Ann(inner, ty) => {
            let ty_t = lower(scope, env, ty)?;
            if let ExprKind::Pair(a, b) = &inner.kind {
                let a_t = lower(scope, env, a)?;
                let b_t = lower(scope, env, b)?;
                return Ok(term::pair(a_t, b_t, ty_t));
            }
            lower(scope, env, inner)
        }
        ExprKind::InductiveDecl(name, params, ty, constructors) => {
            // Check for duplicate
            if env.get_inductive(name).is_some() {
                return Err(LowerError::DuplicateInductive(name.clone()));
            }

            // Lower parameter types
            let mut param_tys = Vec::new();
            let mut param_scope = scope.clone();
            for (param_name, param_ty) in params {
                let param_ty_t = lower(&param_scope, env, param_ty)?;
                param_tys.push(param_ty_t);
                param_scope = param_scope.push(param_name);
            }

            // Lower the type of the inductive (should be a universe)
            let _ind_ty = lower(&param_scope, env, ty)?;

            // Lower constructors
            let mut constructor_defs = Vec::new();
            let mut seen_constructors = HashMap::new();

            for cdecl in constructors {
                // Constructor is encoded as ConApp("_", name, [ty])
                if let ExprKind::ConApp(_, con_name, args) = &cdecl.kind {
                    if args.len() != 1 {
                        return Err(LowerError::InvalidConstructorType);
                    }
                    let con_ty_expr = &args[0];
                    if seen_constructors.contains_key(con_name) {
                        return Err(LowerError::DuplicateConstructor(
                            name.clone(),
                            con_name.clone(),
                        ));
                    }
                    seen_constructors.insert(con_name.clone(), ());

                    // Lower constructor type in a scope with the parameters
                    let con_ty = lower(&param_scope, env, con_ty_expr)?;

                    // Verify constructor type ends in the inductive type applied to params
                    // (simplified check - in practice would need full verification)
                    constructor_defs.push(ConstructorDef {
                        name: con_name.clone(),
                        ty: con_ty,
                    });
                }
            }

            // For now, indices are empty (non-indexed inductives)
            let inductive = InductiveDef {
                name: name.clone(),
                params: param_tys.clone(),
                indices: Vec::new(),
                constructors: constructor_defs,
            };

            env.insert_inductive(inductive);

            // Return the inductive type applied to its parameters
            Ok(term::inductive(name.clone(), param_tys))
        }
        ExprKind::ConApp(ind_name, con_name, args) => {
            let arg_terms: Result<Vec<Term>, LowerError> =
                args.iter().map(|a| lower(scope, env, a)).collect();
            Ok(term::con(ind_name.clone(), con_name.clone(), arg_terms?))
        }
        ExprKind::Elim(_ind_name, motive, cases) => {
            // Lower motive
            let _motive_t = lower(scope, env, motive)?;

            // Lower cases
            let mut case_terms = Vec::new();
            for (con_name, case_body) in cases {
                let body_t = lower(scope, env, case_body)?;
                case_terms.push((con_name.clone(), body_t));
            }

            // We need the scrutinee - for now, we'll need it to be provided
            // In a real implementation, this would be more sophisticated
            // For now, return an error or placeholder
            // This is a simplified version - the full implementation would
            // need to handle the scrutinee properly
            Err(LowerError::InvalidConstructorType) // placeholder
        }
    }
}

fn apply_unary(
    scope: &Scope,
    env: &mut GlobalEnv,
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
    Ok(f(lower(scope, env, args[0])?))
}

fn apply_quaternary(
    scope: &Scope,
    env: &mut GlobalEnv,
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
        lower(scope, env, args[0])?,
        lower(scope, env, args[1])?,
        lower(scope, env, args[2])?,
        lower(scope, env, args[3])?,
    ))
}
