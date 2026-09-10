use tpt_rustir_core::{check, conv, infer, normalize, term, Ctx, GlobalEnv};

fn env() -> GlobalEnv {
    GlobalEnv::new()
}

#[test]
fn identity_function_infers_pi_type() {
    let e = env();
    let ctx: Ctx = vec![];
    // \x : Type0 . x  :  Pi (x : Type0) . Type0
    let id = term::lambda(term::universe(0), term::var(0));
    let ty = infer(&e, &ctx, &id).unwrap();
    let expected = term::pi(term::universe(0), term::universe(0));
    assert!(conv(&e, &ty, &expected));
}

#[test]
fn beta_reduction_applies_identity() {
    let e = env();
    let ctx: Ctx = vec![];
    let id = term::lambda(term::nat(), term::var(0));
    let applied = term::app(id, term::zero());
    let ty = infer(&e, &ctx, &applied).unwrap();
    assert!(conv(&e, &ty, &term::nat()));
    let nf = normalize(&e, &applied);
    assert!(conv(&e, &nf, &term::zero()));
}

#[test]
fn sigma_pair_and_projections() {
    let e = env();
    let ctx: Ctx = vec![];
    // Sigma (_: Nat), Nat  — non-dependent pair (Nat, Nat)
    let sigma_ty = term::sigma(term::nat(), term::nat());
    let one = term::succ(term::zero());
    let p = term::pair(term::zero(), one.clone(), sigma_ty.clone());
    let ty = infer(&e, &ctx, &p).unwrap();
    assert!(conv(&e, &ty, &sigma_ty));

    let fst_ty = infer(&e, &ctx, &term::fst(p.clone())).unwrap();
    assert!(conv(&e, &fst_ty, &term::nat()));

    let snd_nf = normalize(&e, &term::snd(p));
    assert!(conv(&e, &snd_nf, &one));
}

#[test]
fn pi_type_checks_as_universe() {
    let e = env();
    let ctx: Ctx = vec![];
    let ty = term::pi(term::nat(), term::bool_ty());
    let sort = infer(&e, &ctx, &ty).unwrap();
    assert!(conv(&e, &sort, &term::universe(0)));
}

#[test]
fn nat_rec_computes_addition_by_one_step() {
    let e = env();
    let ctx: Ctx = vec![];
    // motive = \_ : Nat . Nat
    let motive = term::lambda(term::nat(), term::nat());
    // step = \n : Nat . \h : Nat . succ h  (i.e. succ (m + n) style successor step)
    let step = term::lambda(
        term::nat(),
        term::lambda(term::nat(), term::succ(term::var(0))),
    );
    // nat_rec computing succ(succ(zero)) from base = succ(zero), one step
    let base = term::succ(term::zero());
    let scrutinee = term::succ(term::zero());
    let rec = term::nat_rec(motive, base, step, scrutinee);
    let ty = infer(&e, &ctx, &rec).unwrap();
    assert!(conv(&e, &ty, &term::nat()));
    let nf = normalize(&e, &rec);
    let expected = term::succ(term::succ(term::zero()));
    assert!(conv(&e, &nf, &expected));
}

#[test]
fn bool_rec_selects_branch() {
    let e = env();
    let motive = term::lambda(term::bool_ty(), term::nat());
    let rec = term::bool_rec(
        motive,
        term::zero(),
        term::succ(term::zero()),
        term::true_(),
    );
    let nf = normalize(&e, &rec);
    assert!(conv(&e, &nf, &term::zero()));
}

#[test]
fn ill_typed_application_is_rejected() {
    let e = env();
    let ctx: Ctx = vec![];
    let not_a_function = term::zero();
    let bad = term::app(not_a_function, term::zero());
    assert!(infer(&e, &ctx, &bad).is_err());
}

#[test]
fn check_mode_rejects_mismatched_annotation() {
    let e = env();
    let ctx: Ctx = vec![];
    let lam = term::lambda(term::nat(), term::var(0));
    // lam : Nat -> Nat is fine, but Nat -> Bool should fail.
    assert!(check(&e, &ctx, &lam, &term::pi(term::nat(), term::nat())).is_ok());
    assert!(check(&e, &ctx, &lam, &term::pi(term::nat(), term::bool_ty())).is_err());
}

#[test]
fn delta_reduction_unfolds_global_definitions() {
    let mut e = env();
    e.insert("two", term::nat(), term::succ(term::succ(term::zero())));
    let ctx: Ctx = vec![];
    let c = term::const_("two");
    let ty = infer(&e, &ctx, &c).unwrap();
    assert!(conv(&e, &ty, &term::nat()));
    let nf = normalize(&e, &c);
    assert!(conv(&e, &nf, &term::succ(term::succ(term::zero()))));
}
