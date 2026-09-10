//! Term representation: plain de Bruijn indices, hash-consed via a thread-local interner.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A hash-consed term. Two structurally equal terms are always the same `Rc`,
/// so equality and hashing of interned terms are pointer operations.
pub type Term = Rc<TermKind>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TermKind {
    /// Bound variable, de Bruijn index counting binders from the innermost outward.
    Var(usize),
    /// A global constant reference, resolved against the environment (delta reduction).
    Const(String),
    Universe(usize),
    Pi(Term, Term),
    Lambda(Term, Term),
    App(Term, Term),
    Sigma(Term, Term),
    /// `Pair(fst, snd, sigma_type)` — annotated so pairs can be inferred, not just checked.
    Pair(Term, Term, Term),
    Fst(Term),
    Snd(Term),
    Nat,
    Zero,
    Succ(Term),
    /// `NatRec(motive, base, step, scrutinee)`.
    NatRec(Term, Term, Term, Term),
    Bool,
    True,
    False,
    /// `BoolRec(motive, case_true, case_false, scrutinee)`.
    BoolRec(Term, Term, Term, Term),
    Let(Term, Term, Term),
}

thread_local! {
    static INTERNER: RefCell<HashMap<TermKind, Term>> = RefCell::new(HashMap::new());
}

/// Intern a `TermKind`, returning the canonical hash-consed `Term` for it.
pub fn mk(kind: TermKind) -> Term {
    INTERNER.with(|tbl| {
        let mut tbl = tbl.borrow_mut();
        if let Some(existing) = tbl.get(&kind) {
            return existing.clone();
        }
        let term = Rc::new(kind.clone());
        tbl.insert(kind, term.clone());
        term
    })
}

pub fn var(n: usize) -> Term {
    mk(TermKind::Var(n))
}
pub fn const_(name: impl Into<String>) -> Term {
    mk(TermKind::Const(name.into()))
}
pub fn universe(n: usize) -> Term {
    mk(TermKind::Universe(n))
}
pub fn pi(dom: Term, cod: Term) -> Term {
    mk(TermKind::Pi(dom, cod))
}
pub fn lambda(dom: Term, body: Term) -> Term {
    mk(TermKind::Lambda(dom, body))
}
pub fn app(f: Term, a: Term) -> Term {
    mk(TermKind::App(f, a))
}
pub fn sigma(fst_ty: Term, snd_ty: Term) -> Term {
    mk(TermKind::Sigma(fst_ty, snd_ty))
}
pub fn pair(fst: Term, snd: Term, ty: Term) -> Term {
    mk(TermKind::Pair(fst, snd, ty))
}
pub fn fst(p: Term) -> Term {
    mk(TermKind::Fst(p))
}
pub fn snd(p: Term) -> Term {
    mk(TermKind::Snd(p))
}
pub fn nat() -> Term {
    mk(TermKind::Nat)
}
pub fn zero() -> Term {
    mk(TermKind::Zero)
}
pub fn succ(n: Term) -> Term {
    mk(TermKind::Succ(n))
}
pub fn nat_rec(motive: Term, base: Term, step: Term, scrutinee: Term) -> Term {
    mk(TermKind::NatRec(motive, base, step, scrutinee))
}
pub fn bool_ty() -> Term {
    mk(TermKind::Bool)
}
pub fn true_() -> Term {
    mk(TermKind::True)
}
pub fn false_() -> Term {
    mk(TermKind::False)
}
pub fn bool_rec(motive: Term, ct: Term, cf: Term, scrutinee: Term) -> Term {
    mk(TermKind::BoolRec(motive, ct, cf, scrutinee))
}
pub fn let_(ty: Term, val: Term, body: Term) -> Term {
    mk(TermKind::Let(ty, val, body))
}

/// Shift free (unbound-at-this-depth) variables in `t` by `amount`, treating
/// indices `< cutoff` as bound within `t` itself and thus left alone.
pub fn shift(t: &Term, cutoff: usize, amount: i64) -> Term {
    match &**t {
        TermKind::Var(n) => {
            if *n >= cutoff {
                var(((*n as i64) + amount) as usize)
            } else {
                t.clone()
            }
        }
        TermKind::Const(_)
        | TermKind::Universe(_)
        | TermKind::Nat
        | TermKind::Zero
        | TermKind::Bool
        | TermKind::True
        | TermKind::False => t.clone(),
        TermKind::Pi(dom, cod) => pi(shift(dom, cutoff, amount), shift(cod, cutoff + 1, amount)),
        TermKind::Lambda(dom, body) => {
            lambda(shift(dom, cutoff, amount), shift(body, cutoff + 1, amount))
        }
        TermKind::App(f, a) => app(shift(f, cutoff, amount), shift(a, cutoff, amount)),
        TermKind::Sigma(a, b) => sigma(shift(a, cutoff, amount), shift(b, cutoff + 1, amount)),
        TermKind::Pair(a, b, ty) => pair(
            shift(a, cutoff, amount),
            shift(b, cutoff, amount),
            shift(ty, cutoff, amount),
        ),
        TermKind::Fst(p) => fst(shift(p, cutoff, amount)),
        TermKind::Snd(p) => snd(shift(p, cutoff, amount)),
        TermKind::Succ(n) => succ(shift(n, cutoff, amount)),
        TermKind::NatRec(m, b, s, t0) => nat_rec(
            shift(m, cutoff, amount),
            shift(b, cutoff, amount),
            shift(s, cutoff, amount),
            shift(t0, cutoff, amount),
        ),
        TermKind::BoolRec(m, ct, cf, t0) => bool_rec(
            shift(m, cutoff, amount),
            shift(ct, cutoff, amount),
            shift(cf, cutoff, amount),
            shift(t0, cutoff, amount),
        ),
        TermKind::Let(ty, val, body) => let_(
            shift(ty, cutoff, amount),
            shift(val, cutoff, amount),
            shift(body, cutoff + 1, amount),
        ),
    }
}

/// Substitute `image` for the variable bound at `depth`, removing that binder.
/// Used for beta reduction: `subst(body, 0, arg)` implements `(\.body) arg`.
pub fn subst(t: &Term, depth: usize, image: &Term) -> Term {
    match &**t {
        TermKind::Var(n) => {
            if *n == depth {
                shift(image, 0, depth as i64)
            } else if *n > depth {
                var(n - 1)
            } else {
                t.clone()
            }
        }
        TermKind::Const(_)
        | TermKind::Universe(_)
        | TermKind::Nat
        | TermKind::Zero
        | TermKind::Bool
        | TermKind::True
        | TermKind::False => t.clone(),
        TermKind::Pi(dom, cod) => pi(subst(dom, depth, image), subst(cod, depth + 1, image)),
        TermKind::Lambda(dom, body) => {
            lambda(subst(dom, depth, image), subst(body, depth + 1, image))
        }
        TermKind::App(f, a) => app(subst(f, depth, image), subst(a, depth, image)),
        TermKind::Sigma(a, b) => sigma(subst(a, depth, image), subst(b, depth + 1, image)),
        TermKind::Pair(a, b, ty) => pair(
            subst(a, depth, image),
            subst(b, depth, image),
            subst(ty, depth, image),
        ),
        TermKind::Fst(p) => fst(subst(p, depth, image)),
        TermKind::Snd(p) => snd(subst(p, depth, image)),
        TermKind::Succ(n) => succ(subst(n, depth, image)),
        TermKind::NatRec(m, b, s, t0) => nat_rec(
            subst(m, depth, image),
            subst(b, depth, image),
            subst(s, depth, image),
            subst(t0, depth, image),
        ),
        TermKind::BoolRec(m, ct, cf, t0) => bool_rec(
            subst(m, depth, image),
            subst(ct, depth, image),
            subst(cf, depth, image),
            subst(t0, depth, image),
        ),
        TermKind::Let(ty, val, body) => let_(
            subst(ty, depth, image),
            subst(val, depth, image),
            subst(body, depth + 1, image),
        ),
    }
}

/// `subst_top(body, arg)` = `body` with its outermost bound variable replaced by `arg`.
pub fn subst_top(body: &Term, arg: &Term) -> Term {
    subst(body, 0, arg)
}
