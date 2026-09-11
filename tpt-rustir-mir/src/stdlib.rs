//! The arithmetic `stdlib` of the MIR bridge: kernel constants (`add`, `mul`,
//! `minus`, `pred`, `is_zero`, `le`) defined by Nat/Bool recursion so the
//! delta/iota reducer can unfold them, plus aliases mapping Rust primitive
//! types onto kernel `Nat`.
//!
//! These definitions are *trusted* in the same sense the kernel is: they are
//! ordinary closed terms, type-checked by the kernel, and every rule the
//! solver assumes about them (`add-succ-l`, `le-succ-succ`, …) is faithful to
//! what these terms compute.

use tpt_rustir_core::env::GlobalEnv;
use tpt_rustir_core::term::{self, Term};

/// The Church numeral `n` in the kernel's `Nat` (`succ^n zero`).
pub fn num(n: u64) -> Term {
    let mut t = term::zero();
    for _ in 0..n {
        t = term::succ(t);
    }
    t
}

/// `pred : Nat -> Nat`; `pred zero = zero`, `pred (succ n) = n`.
fn pred_value() -> Term {
    // \n : Nat. natrec(\_ : Nat. Nat, zero, \n' : Nat. \h : Nat. n', n)
    term::lambda(
        term::nat(),
        term::nat_rec(
            term::lambda(term::nat(), term::nat()),
            term::zero(),
            // step: return the predecessor argument n' (Var(1) under two binders)
            term::lambda(term::nat(), term::lambda(term::nat(), term::var(1))),
            term::var(0),
        ),
    )
}

/// `add : Nat -> Nat -> Nat`; `add zero b = b`, `add (succ a) b = succ (add a b)`
/// — recursion on the first argument, matching the solver's `add-*-l` rules.
fn add_value() -> Term {
    term::lambda(
        term::nat(),
        term::lambda(
            term::nat(),
            // \a : Nat. \b : Nat. natrec(\_ . Nat, b, \n. \h. succ h, a)
            term::nat_rec(
                term::lambda(term::nat(), term::nat()),
                term::var(0), // b
                term::lambda(
                    term::nat(),
                    term::lambda(term::nat(), term::succ(term::var(0))),
                ),
                term::var(1), // a
            ),
        ),
    )
}

/// `mul : Nat -> Nat -> Nat`; `mul zero b = zero`,
/// `mul (succ a) b = add b (mul a b)` — matching the solver's `mul-succ-l`.
fn mul_value() -> Term {
    term::lambda(
        term::nat(),
        term::lambda(
            term::nat(),
            term::nat_rec(
                term::lambda(term::nat(), term::nat()),
                term::zero(),
                // step: \n : Nat. \h : Nat. add b h  (b = Var(2), h = Var(0))
                term::lambda(
                    term::nat(),
                    term::lambda(
                        term::nat(),
                        term::app(
                            term::app(term::const_("add"), term::var(2)),
                            term::var(0),
                        ),
                    ),
                ),
                term::var(1), // a
            ),
        ),
    )
}
/// `minus : Nat -> Nat -> Nat`, saturating: `minus a zero = a`,
/// `minus a (succ b) = pred (minus a b)`.
fn minus_value() -> Term {
    term::lambda(
        term::nat(),
        term::lambda(
            term::nat(),
            // \a : Nat. \b : Nat. natrec(\_ . Nat, a, \n. \h. pred h, b)
            term::nat_rec(
                term::lambda(term::nat(), term::nat()),
                term::var(1), // a
                term::lambda(
                    term::nat(),
                    term::lambda(
                        term::nat(),
                        term::app(term::const_("pred"), term::var(0)),
                    ),
                ),
                term::var(0), // b
            ),
        ),
    )
}

/// `is_zero : Nat -> Bool`; `is_zero zero = true`, `is_zero (succ n) = false`.
fn is_zero_value() -> Term {
    term::lambda(
        term::nat(),
        term::nat_rec(
            term::lambda(term::nat(), term::bool_ty()),
            term::true_(),
            term::lambda(term::nat(), term::lambda(term::nat(), term::false_())),
            term::var(0),
        ),
    )
}

/// `le : Nat -> Nat -> Bool` as `is_zero (minus a b)` — saturating
/// subtraction makes `le a b` true exactly when `a <= b` over Nat.
fn le_value() -> Term {
    term::lambda(
        term::nat(),
        term::lambda(
            term::nat(),
            term::app(
                term::const_("is_zero"),
                term::app(
                    term::app(term::const_("minus"), term::var(1)),
                    term::var(0),
                ),
            ),
        ),
    )
}

/// Rust primitive names aliased onto kernel `Nat` (numeric reasoning is over
/// Nat for 1.0; signed overflow/wraparound is not modeled — see the crate
/// docs). The alias names match what `RustType::to_core_term` generates
/// (`Nat`/`Int` + the primitive name). `bool` maps to the kernel `Bool`
/// directly via `RustType`.
const NAT_ALIASES: &[(&str, &str)] = &[
    ("Natu8", "u8"),
    ("Natu16", "u16"),
    ("Natu32", "u32"),
    ("Natu64", "u64"),
    ("Natu128", "u128"),
    ("Natusize", "usize"),
    ("Inti8", "i8"),
    ("Inti16", "i16"),
    ("Inti32", "i32"),
    ("Inti64", "i64"),
    ("Inti128", "i128"),
    ("Intisize", "isize"),
    ("Char", "char"),
];

/// Build the MIR bridge's global environment: the arithmetic stdlib plus the
/// primitive-type aliases.
pub fn stdlib() -> GlobalEnv {
    let mut env = GlobalEnv::new();

    let nat_to_nat = term::pi(term::nat(), term::nat());
    let nat_to_nat_to_nat = term::pi(term::nat(), nat_to_nat.clone());
    let nat_to_nat_to_bool = term::pi(term::nat(), term::pi(term::nat(), term::bool_ty()));

    env.insert("pred", nat_to_nat.clone(), pred_value());
    env.insert("add", nat_to_nat_to_nat.clone(), add_value());
    env.insert("mul", nat_to_nat_to_nat.clone(), mul_value());
    env.insert("minus", nat_to_nat_to_nat.clone(), minus_value());
    env.insert(
        "is_zero",
        term::pi(term::nat(), term::bool_ty()),
        is_zero_value(),
    );
    env.insert("le", nat_to_nat_to_bool, le_value());

    // Primitive aliases: `Const("NatU32")` etc. unfold (delta) to kernel Nat,
    // so arithmetic over Rust integers type-checks against kernel Nat.
    for (alias, _) in NAT_ALIASES {
        env.insert(alias.to_string(), term::universe(0), term::nat());
    }

    env
}

/// The Rust primitive name an alias stands for, if any.
pub fn alias_source_name(alias: &str) -> Option<&'static str> {
    NAT_ALIASES
        .iter()
        .find(|(a, _)| *a == alias)
        .map(|(_, src)| *src)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_rustir_core::{conv, infer, normalize, Ctx};

    #[test]
    fn stdlib_computes_arithmetic() {
        let env = stdlib();
        let ctx: Ctx = vec![];
        // add 2 3 = 5
        let t = term::app(term::app(term::const_("add"), num(2)), num(3));
        infer(&env, &ctx, &t).unwrap();
        assert!(conv(&env, &normalize(&env, &t), &num(5)));
        // mul 2 3 = 6
        let t = term::app(term::app(term::const_("mul"), num(2)), num(3));
        assert!(conv(&env, &normalize(&env, &t), &num(6)));
    }

    #[test]
    fn stdlib_le_matches_nat_order() {
        let env = stdlib();
        let le = |a: u64, b: u64| {
            let t = term::app(term::app(term::const_("le"), num(a)), num(b));
            normalize(&env, &t)
        };
        assert!(conv(&env, &le(3, 5), &term::true_()));
        assert!(conv(&env, &le(5, 5), &term::true_()));
        assert!(conv(&env, &le(6, 5), &term::false_()));
        assert!(conv(&env, &le(0, 0), &term::true_()));
    }

    #[test]
    fn aliases_unfold_to_nat() {
        let env = stdlib();
        // As a *type*, `Natu32` is convertible with kernel Nat (locals
        // declared `u32` verify as Nat).
        assert!(conv(
            &env,
            &term::nat(),
            &term::const_("Natu32")
        ));
        // ... and arithmetic over such locals type-checks.
        let ctx: Ctx = vec![term::const_("Natu32")];
        let ty = infer(&env, &ctx, &term::var(0)).unwrap();
        assert!(conv(&env, &ty, &term::nat()));
        let added = term::app(term::const_("add"), term::var(0));
        let ty = infer(&env, &ctx, &added).unwrap();
        assert!(conv(&env, &ty, &term::pi(term::nat(), term::nat())));
    }
}