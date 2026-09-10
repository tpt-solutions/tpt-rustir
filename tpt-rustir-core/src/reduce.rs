//! Term reduction: beta, delta, and iota reduction, plus normalization and
//! algorithmic conversion checking (definitional equality).

use crate::env::GlobalEnv;
use crate::term::{self, Term, TermKind};

/// Reduce `t` one step toward weak-head normal form, or return `None` if it is
/// already in whnf. Performs beta, delta, and iota reduction.
pub fn whnf_step(env: &GlobalEnv, t: &Term) -> Option<Term> {
    match &**t {
        TermKind::Const(name) => env.get(name).map(|def| def.value.clone()),
        TermKind::App(f, a) => {
            if let Some(f2) = whnf_step(env, f) {
                return Some(term::app(f2, a.clone()));
            }
            if let TermKind::Lambda(_, body) = &**f {
                return Some(term::subst_top(body, a));
            }
            None
        }
        TermKind::Fst(p) => {
            if let Some(p2) = whnf_step(env, p) {
                return Some(term::fst(p2));
            }
            if let TermKind::Pair(a, _, _) = &**p {
                return Some(a.clone());
            }
            None
        }
        TermKind::Snd(p) => {
            if let Some(p2) = whnf_step(env, p) {
                return Some(term::snd(p2));
            }
            if let TermKind::Pair(_, b, _) = &**p {
                return Some(b.clone());
            }
            None
        }
        TermKind::NatRec(motive, base, step, scrutinee) => {
            if let Some(s2) = whnf_step(env, scrutinee) {
                return Some(term::nat_rec(
                    motive.clone(),
                    base.clone(),
                    step.clone(),
                    s2,
                ));
            }
            match &**scrutinee {
                TermKind::Zero => Some(base.clone()),
                TermKind::Succ(pred) => {
                    let rec =
                        term::nat_rec(motive.clone(), base.clone(), step.clone(), pred.clone());
                    Some(term::app(term::app(step.clone(), pred.clone()), rec))
                }
                _ => None,
            }
        }
        TermKind::BoolRec(motive, ct, cf, scrutinee) => {
            if let Some(s2) = whnf_step(env, scrutinee) {
                return Some(term::bool_rec(motive.clone(), ct.clone(), cf.clone(), s2));
            }
            match &**scrutinee {
                TermKind::True => Some(ct.clone()),
                TermKind::False => Some(cf.clone()),
                _ => None,
            }
        }
        TermKind::Let(_, val, body) => Some(term::subst_top(body, val)),
        _ => None,
    }
}

pub fn whnf(env: &GlobalEnv, t: &Term) -> Term {
    let mut cur = t.clone();
    while let Some(next) = whnf_step(env, &cur) {
        cur = next;
    }
    cur
}

/// Fully normalize `t` (normal form under all binders), used for display/testing.
pub fn normalize(env: &GlobalEnv, t: &Term) -> Term {
    let t = whnf(env, t);
    match &*t {
        TermKind::Pi(dom, cod) => term::pi(normalize(env, dom), normalize(env, cod)),
        TermKind::Lambda(dom, body) => term::lambda(normalize(env, dom), normalize(env, body)),
        TermKind::App(f, a) => term::app(normalize(env, f), normalize(env, a)),
        TermKind::Sigma(a, b) => term::sigma(normalize(env, a), normalize(env, b)),
        TermKind::Pair(a, b, ty) => {
            term::pair(normalize(env, a), normalize(env, b), normalize(env, ty))
        }
        TermKind::Fst(p) => term::fst(normalize(env, p)),
        TermKind::Snd(p) => term::snd(normalize(env, p)),
        TermKind::Succ(n) => term::succ(normalize(env, n)),
        TermKind::NatRec(m, b, s, t0) => term::nat_rec(
            normalize(env, m),
            normalize(env, b),
            normalize(env, s),
            normalize(env, t0),
        ),
        TermKind::BoolRec(m, ct, cf, t0) => term::bool_rec(
            normalize(env, m),
            normalize(env, ct),
            normalize(env, cf),
            normalize(env, t0),
        ),
        TermKind::Let(ty, val, body) => term::let_(
            normalize(env, ty),
            normalize(env, val),
            normalize(env, body),
        ),
        _ => t,
    }
}

/// Algorithmic definitional equality: compare weak-head normal forms structurally,
/// recursing into subterms (up to alpha-equivalence, which de Bruijn gives for free).
pub fn conv(env: &GlobalEnv, a: &Term, b: &Term) -> bool {
    if std::rc::Rc::ptr_eq(a, b) {
        return true;
    }
    let a = whnf(env, a);
    let b = whnf(env, b);
    if std::rc::Rc::ptr_eq(&a, &b) {
        return true;
    }
    match (&*a, &*b) {
        (TermKind::Var(n1), TermKind::Var(n2)) => n1 == n2,
        (TermKind::Const(n1), TermKind::Const(n2)) => n1 == n2,
        (TermKind::Universe(i), TermKind::Universe(j)) => i == j,
        (TermKind::Nat, TermKind::Nat)
        | (TermKind::Zero, TermKind::Zero)
        | (TermKind::Bool, TermKind::Bool)
        | (TermKind::True, TermKind::True)
        | (TermKind::False, TermKind::False) => true,
        (TermKind::Pi(d1, c1), TermKind::Pi(d2, c2)) => conv(env, d1, d2) && conv(env, c1, c2),
        (TermKind::Lambda(d1, b1), TermKind::Lambda(d2, b2)) => {
            conv(env, d1, d2) && conv(env, b1, b2)
        }
        (TermKind::App(f1, a1), TermKind::App(f2, a2)) => conv(env, f1, f2) && conv(env, a1, a2),
        (TermKind::Sigma(a1, b1), TermKind::Sigma(a2, b2)) => {
            conv(env, a1, a2) && conv(env, b1, b2)
        }
        (TermKind::Pair(a1, b1, _), TermKind::Pair(a2, b2, _)) => {
            conv(env, a1, a2) && conv(env, b1, b2)
        }
        (TermKind::Fst(p1), TermKind::Fst(p2)) => conv(env, p1, p2),
        (TermKind::Snd(p1), TermKind::Snd(p2)) => conv(env, p1, p2),
        (TermKind::Succ(n1), TermKind::Succ(n2)) => conv(env, n1, n2),
        (TermKind::NatRec(m1, b1, s1, t1), TermKind::NatRec(m2, b2, s2, t2)) => {
            conv(env, m1, m2) && conv(env, b1, b2) && conv(env, s1, s2) && conv(env, t1, t2)
        }
        (TermKind::BoolRec(m1, ct1, cf1, t1), TermKind::BoolRec(m2, ct2, cf2, t2)) => {
            conv(env, m1, m2) && conv(env, ct1, ct2) && conv(env, cf1, cf2) && conv(env, t1, t2)
        }
        _ => false,
    }
}
