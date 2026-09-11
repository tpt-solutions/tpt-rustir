//! Bidirectional type-checking: `infer` synthesizes a type, `check` verifies
//! against an expected type via conversion checking.

use std::fmt;

use crate::env::GlobalEnv;
use crate::reduce::{conv, whnf};
use crate::term::{self, Term, TermKind};

pub type Ctx = Vec<Term>;

#[derive(Debug, Clone)]
pub enum TypeError {
    UnboundVar(usize),
    UnknownConst(String),
    NotAFunctionType(Term),
    NotAPairType(Term),
    NotAUniverse(Term),
    NotNat(Term),
    NotBool(Term),
    UnknownInductive(String),
    UnknownConstructor(String, String), // (inductive_name, constructor_name)
    Mismatch { expected: Term, found: Term },
    CannotInfer(&'static str),
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeError::UnboundVar(n) => write!(f, "unbound variable index {n}"),
            TypeError::UnknownConst(n) => write!(f, "unknown constant `{n}`"),
            TypeError::NotAFunctionType(t) => write!(f, "expected a Pi type, found {t:?}"),
            TypeError::NotAPairType(t) => write!(f, "expected a Sigma type, found {t:?}"),
            TypeError::NotAUniverse(t) => write!(f, "expected a universe, found {t:?}"),
            TypeError::NotNat(t) => write!(f, "expected type Nat, found {t:?}"),
            TypeError::NotBool(t) => write!(f, "expected type Bool, found {t:?}"),
            TypeError::UnknownInductive(n) => write!(f, "unknown inductive type `{n}`"),
            TypeError::UnknownConstructor(ind, con) => {
                write!(f, "unknown constructor `{con}` for inductive `{ind}`")
            }
            TypeError::Mismatch { expected, found } => {
                write!(f, "type mismatch: expected {expected:?}, found {found:?}")
            }
            TypeError::CannotInfer(what) => {
                write!(f, "cannot infer type of {what}; add an annotation")
            }
        }
    }
}

impl std::error::Error for TypeError {}

/// Look up the type of `Var(n)` in `ctx`, shifting it to account for the `n`
/// binders introduced since it was pushed.
fn lookup(ctx: &Ctx, n: usize) -> Result<Term, TypeError> {
    if n >= ctx.len() {
        return Err(TypeError::UnboundVar(n));
    }
    let ty = &ctx[ctx.len() - 1 - n];
    Ok(term::shift(ty, 0, (n + 1) as i64))
}

fn push(ctx: &Ctx, ty: Term) -> Ctx {
    let mut ctx = ctx.clone();
    ctx.push(ty);
    ctx
}

pub fn infer(env: &GlobalEnv, ctx: &Ctx, t: &Term) -> Result<Term, TypeError> {
    match &**t {
        TermKind::Var(n) => lookup(ctx, *n),
        TermKind::Const(name) => env
            .get(name)
            .map(|def| def.ty.clone())
            .ok_or_else(|| TypeError::UnknownConst(name.clone())),
        TermKind::Universe(i) => Ok(term::universe(i + 1)),
        TermKind::Nat | TermKind::Bool => Ok(term::universe(0)),
        TermKind::Zero => Ok(term::nat()),
        TermKind::Succ(n) => {
            check(env, ctx, n, &term::nat())?;
            Ok(term::nat())
        }
        TermKind::True | TermKind::False => Ok(term::bool_ty()),
        TermKind::Pi(dom, cod) => {
            let dom_ty = infer(env, ctx, dom)?;
            expect_universe(env, &dom_ty)?;
            let cod_ctx = push(ctx, dom.clone());
            let cod_ty = infer(env, &cod_ctx, cod)?;
            expect_universe(env, &cod_ty)
        }
        TermKind::Sigma(a, b) => {
            let a_ty = infer(env, ctx, a)?;
            expect_universe(env, &a_ty)?;
            let b_ctx = push(ctx, a.clone());
            let b_ty = infer(env, &b_ctx, b)?;
            expect_universe(env, &b_ty)
        }
        TermKind::Lambda(dom, body) => {
            let dom_ty = infer(env, ctx, dom)?;
            expect_universe(env, &dom_ty)?;
            let body_ctx = push(ctx, dom.clone());
            let body_ty = infer(env, &body_ctx, body)?;
            Ok(term::pi(dom.clone(), body_ty))
        }
        TermKind::App(f, a) => {
            let f_ty = infer(env, ctx, f)?;
            let f_ty_whnf = whnf(env, &f_ty);
            match &*f_ty_whnf {
                TermKind::Pi(dom, cod) => {
                    check(env, ctx, a, dom)?;
                    Ok(term::subst_top(cod, a))
                }
                _ => Err(TypeError::NotAFunctionType(f_ty_whnf)),
            }
        }
        TermKind::Pair(a, b, ty) => {
            let ty_whnf = whnf(env, ty);
            match &*ty_whnf {
                TermKind::Sigma(a_ty, b_ty) => {
                    check(env, ctx, a, a_ty)?;
                    let b_ty_inst = term::subst_top(b_ty, a);
                    check(env, ctx, b, &b_ty_inst)?;
                    Ok(ty.clone())
                }
                _ => Err(TypeError::NotAPairType(ty_whnf)),
            }
        }
        TermKind::Fst(p) => {
            let p_ty = infer(env, ctx, p)?;
            let p_ty_whnf = whnf(env, &p_ty);
            match &*p_ty_whnf {
                TermKind::Sigma(a_ty, _) => Ok(a_ty.clone()),
                _ => Err(TypeError::NotAPairType(p_ty_whnf)),
            }
        }
        TermKind::Snd(p) => {
            let p_ty = infer(env, ctx, p)?;
            let p_ty_whnf = whnf(env, &p_ty);
            match &*p_ty_whnf {
                TermKind::Sigma(_, b_ty) => Ok(term::subst_top(b_ty, &term::fst(p.clone()))),
                _ => Err(TypeError::NotAPairType(p_ty_whnf)),
            }
        }
        TermKind::NatRec(motive, base, step, scrutinee) => {
            check(env, ctx, scrutinee, &term::nat())?;
            check_motive(env, ctx, motive, &term::nat())?;
            let base_ty_expected = term::app(motive.clone(), term::zero());
            check(env, ctx, base, &base_ty_expected)?;
            // step : (n : Nat) -> motive n -> motive (succ n)
            let hyp_ty = term::app(term::shift(motive, 0, 1), term::var(0));
            let succ_ty = term::app(term::shift(motive, 0, 2), term::succ(term::var(1)));
            let step_expected = term::pi(term::nat(), term::pi(hyp_ty, succ_ty));
            check(env, ctx, step, &step_expected)?;
            Ok(term::app(motive.clone(), scrutinee.clone()))
        }
        TermKind::BoolRec(motive, ct, cf, scrutinee) => {
            check(env, ctx, scrutinee, &term::bool_ty())?;
            check_motive(env, ctx, motive, &term::bool_ty())?;
            let ct_expected = term::app(motive.clone(), term::true_());
            check(env, ctx, ct, &ct_expected)?;
            let cf_expected = term::app(motive.clone(), term::false_());
            check(env, ctx, cf, &cf_expected)?;
            Ok(term::app(motive.clone(), scrutinee.clone()))
        }
        TermKind::Inductive(name, params) => {
            let ind = env
                .get_inductive(name)
                .ok_or_else(|| TypeError::UnknownInductive(name.clone()))?;
            // Check all parameters are well-typed
            let mut param_ctx = ctx.clone();
            for (i, param_ty) in ind.params.iter().enumerate() {
                check(env, &param_ctx, &params[i], param_ty)?;
                param_ctx.push(params[i].clone());
            }
            // Inductive type itself lives in the universe of its indices
            // For simplicity, we return Type0 (can be refined later)
            Ok(term::universe(0))
        }
        TermKind::Con(ind_name, con_name, args) => {
            let con_def = env
                .get_constructor(ind_name, con_name)
                .ok_or_else(|| TypeError::UnknownConstructor(ind_name.clone(), con_name.clone()))?;
            // The constructor type is a Pi type ending in the inductive type
            // We need to check each argument against the corresponding domain
            let mut con_ty = whnf(env, &con_def.ty);
            let mut arg_idx = 0;
            while let TermKind::Pi(dom, cod) = &*con_ty {
                if arg_idx >= args.len() {
                    break; // partially applied constructor - this is valid
                }
                check(env, ctx, &args[arg_idx], dom)?;
                con_ty = term::subst_top(cod, &args[arg_idx]);
                arg_idx += 1;
            }
            if arg_idx != args.len() {
                return Err(TypeError::CannotInfer("too many arguments to constructor"));
            }
            Ok(con_ty)
        }
        TermKind::IndRec(ind_name, motive, cases, scrutinee) => {
            let ind = env
                .get_inductive(ind_name)
                .ok_or_else(|| TypeError::UnknownInductive(ind_name.clone()))?;

            // Check scrutinee has the inductive type (with some parameters)
            let scrut_ty = infer(env, ctx, scrutinee)?;
            let scrut_ty_whnf = whnf(env, &scrut_ty);

            // Extract the inductive type from scrutinee type
            let mut t = scrut_ty_whnf.clone();
            let result = match &*t {
                TermKind::Inductive(n, params) if n == ind_name => (ind, params.clone()),
                TermKind::App(_, _) => {
                    // It could be an applied inductive type
                    let mut params = Vec::new();
                    while let TermKind::App(f, a) = &*t {
                        params.insert(0, a.clone());
                        t = f.clone();
                    }
                    if let TermKind::Inductive(n, base_params) = &*t {
                        if n == ind_name {
                            // Combine base params with applied params
                            let mut all_params = base_params.clone();
                            all_params.extend(params);
                            (ind, all_params)
                        } else {
                            return Err(TypeError::Mismatch {
                                expected: term::inductive(ind_name.clone(), ind.params.clone()),
                                found: scrut_ty_whnf,
                            });
                        }
                    } else {
                        return Err(TypeError::Mismatch {
                            expected: term::inductive(ind_name.clone(), ind.params.clone()),
                            found: scrut_ty_whnf,
                        });
                    }
                }
                _ => {
                    return Err(TypeError::Mismatch {
                        expected: term::inductive(ind_name.clone(), ind.params.clone()),
                        found: scrut_ty_whnf,
                    });
                }
            };
            let _inductive_type = result.0;
            let _param_args = result.1;

            // Check motive: should be (params...) -> (indices...) -> Type
            // For simplicity, we check that motive applied to the scrutinee yields a type
            // Actually, the motive should be: (x : Ind params indices) -> Type_i
            let motive_ty = infer(env, ctx, motive)?;
            let motive_ty_whnf = whnf(env, &motive_ty);
            match &*motive_ty_whnf {
                TermKind::Pi(dom, cod) => {
                    // Check domain matches scrutinee type
                    if !conv(env, dom, &scrut_ty) {
                        return Err(TypeError::Mismatch {
                            expected: scrut_ty,
                            found: dom.clone(),
                        });
                    }
                    expect_universe(env, cod)?;
                }
                _ => return Err(TypeError::NotAFunctionType(motive_ty_whnf)),
            }

            // Check each case
            for (con_name, case_body) in cases {
                let con_def = env.get_constructor(ind_name, con_name).ok_or_else(|| {
                    TypeError::UnknownConstructor(ind_name.clone(), con_name.clone())
                })?;

                // The case body should have type: motive applied to the constructor
                // For each constructor argument, we extend the context
                let mut case_ty = whnf(env, &con_def.ty);
                let mut case_ctx = ctx.clone();
                let mut final_cod = None;
                loop {
                    let case_ty_whnf = whnf(env, &case_ty);
                    if let TermKind::Pi(dom, cod) = &*case_ty_whnf {
                        // Add the constructor argument to context
                        case_ctx.push(dom.clone());
                        case_ty = cod.clone();
                        final_cod = Some(cod.clone());
                    } else {
                        break;
                    }
                }
                // Now case_ty should be the inductive type applied to params and indices
                // We need to apply motive to the constructed term
                // For simplicity, we check the case body in the extended context
                let expected_ty = if let Some(cod_ref) = final_cod {
                    term::subst_top(&cod_ref, scrutinee)
                } else {
                    term::app(motive.clone(), scrutinee.clone())
                };
                check(env, &case_ctx, case_body, &expected_ty)?;
            }

            // Return motive applied to scrutinee
            Ok(term::app(motive.clone(), scrutinee.clone()))
        }
        TermKind::Let(ty, val, body) => {
            check(env, ctx, val, ty)?;
            let body_ctx = push(ctx, ty.clone());
            let body_ty = infer(env, &body_ctx, body)?;
            Ok(term::subst_top(&body_ty, val))
        }
    }
}

/// Check that `motive` has type `domain -> Type_i` for some universe level `i`.
fn check_motive(env: &GlobalEnv, ctx: &Ctx, motive: &Term, domain: &Term) -> Result<(), TypeError> {
    let motive_ty = infer(env, ctx, motive)?;
    let motive_ty_whnf = whnf(env, &motive_ty);
    match &*motive_ty_whnf {
        TermKind::Pi(dom, cod) => {
            if !conv(env, dom, domain) {
                return Err(TypeError::Mismatch {
                    expected: domain.clone(),
                    found: dom.clone(),
                });
            }
            expect_universe(env, cod)?;
            Ok(())
        }
        _ => Err(TypeError::NotAFunctionType(motive_ty_whnf)),
    }
}

fn expect_universe(env: &GlobalEnv, ty: &Term) -> Result<Term, TypeError> {
    let ty_whnf = whnf(env, ty);
    match &*ty_whnf {
        TermKind::Universe(_) => Ok(ty_whnf.clone()),
        _ => Err(TypeError::NotAUniverse(ty_whnf)),
    }
}

pub fn check(env: &GlobalEnv, ctx: &Ctx, t: &Term, expected: &Term) -> Result<(), TypeError> {
    match &**t {
        TermKind::Lambda(dom, body) => {
            let expected_whnf = whnf(env, expected);
            match &*expected_whnf {
                TermKind::Pi(exp_dom, exp_cod) => {
                    if !conv(env, dom, exp_dom) {
                        return Err(TypeError::Mismatch {
                            expected: exp_dom.clone(),
                            found: dom.clone(),
                        });
                    }
                    let body_ctx = push(ctx, dom.clone());
                    check(env, &body_ctx, body, exp_cod)
                }
                _ => Err(TypeError::NotAFunctionType(expected_whnf)),
            }
        }
        TermKind::Pair(a, b, _) => {
            let expected_whnf = whnf(env, expected);
            match &*expected_whnf {
                TermKind::Sigma(a_ty, b_ty) => {
                    check(env, ctx, a, a_ty)?;
                    let b_ty_inst = term::subst_top(b_ty, a);
                    check(env, ctx, b, &b_ty_inst)
                }
                _ => Err(TypeError::NotAPairType(expected_whnf)),
            }
        }
        TermKind::Inductive(name, params) => {
            let ind = env
                .get_inductive(name)
                .ok_or_else(|| TypeError::UnknownInductive(name.clone()))?;
            // Check all parameters are well-typed
            let mut param_ctx = ctx.clone();
            for (i, param_ty) in ind.params.iter().enumerate() {
                check(env, &param_ctx, &params[i], param_ty)?;
                param_ctx.push(params[i].clone());
            }
            // Check the expected type is the inductive type with the same params
            let expected_whnf = whnf(env, expected);
            match &*expected_whnf {
                TermKind::Inductive(expected_name, expected_params) if expected_name == name => {
                    if params.len() != expected_params.len() {
                        return Err(TypeError::Mismatch {
                            expected: term::inductive(name.clone(), expected_params.clone()),
                            found: t.clone(),
                        });
                    }
                    for (p1, p2) in params.iter().zip(expected_params) {
                        if !conv(env, p1, p2) {
                            return Err(TypeError::Mismatch {
                                expected: p2.clone(),
                                found: p1.clone(),
                            });
                        }
                    }
                    Ok(())
                }
                _ => Err(TypeError::Mismatch {
                    expected: term::inductive(name.clone(), ind.params.clone()),
                    found: expected_whnf,
                }),
            }
        }
        TermKind::Con(_ind_name, _con_name, _args) => {
            // In check mode, we just infer and then convert
            let found = infer(env, ctx, t)?;
            if conv(env, &found, expected) {
                Ok(())
            } else {
                Err(TypeError::Mismatch {
                    expected: expected.clone(),
                    found,
                })
            }
        }
        TermKind::IndRec(_ind_name, _motive, _cases, _scrutinee) => {
            // In check mode, we just infer and then convert
            let found = infer(env, ctx, t)?;
            if conv(env, &found, expected) {
                Ok(())
            } else {
                Err(TypeError::Mismatch {
                    expected: expected.clone(),
                    found,
                })
            }
        }
        _ => {
            let found = infer(env, ctx, t)?;
            if conv(env, &found, expected) {
                Ok(())
            } else {
                Err(TypeError::Mismatch {
                    expected: expected.clone(),
                    found,
                })
            }
        }
    }
}
