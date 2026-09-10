use tpt_rustir_core::{infer, normalize, GlobalEnv};
use tpt_rustir_syntax::{lower, parse_and_lower, parser, pretty};

fn infer_and_print(src: &str) -> (String, String) {
    let expr = parser::parse(src).unwrap_or_else(|e| panic!("parse error in {src:?}: {e:?}"));
    let term = lower::lower(&lower::Scope::new(), &expr)
        .unwrap_or_else(|e| panic!("lowering error in {src:?}: {e}"));
    let env = GlobalEnv::new();
    let ctx = Vec::new();
    let ty = infer(&env, &ctx, &term).unwrap_or_else(|e| panic!("type error in {src:?}: {e}"));
    let nf = normalize(&env, &term);
    (pretty::print(&ty), pretty::print(&nf))
}

#[test]
fn basic_lambda_calculus() {
    let (ty, _) = infer_and_print("\\x : Nat => x");
    assert_eq!(ty, "(x : Nat) -> Nat");

    let (ty, nf) = infer_and_print("(\\x : Nat => succ x) 2");
    assert_eq!(ty, "Nat");
    assert_eq!(nf, "succ (succ (succ zero))");
}

#[test]
fn dependent_pi_types() {
    let (ty, _) = infer_and_print("\\A : Type0 => \\x : A => x");
    assert_eq!(ty, "(x : Type0) -> (y : x) -> x");
}

#[test]
fn dependent_sigma_pairs() {
    let (ty, nf) = infer_and_print("((2, true) : (Nat * Bool))");
    assert_eq!(ty, "(x : Nat) * Bool");
    assert_eq!(nf, "(succ (succ zero), true)");

    let (ty, _) = infer_and_print("fst ((2, true) : (Nat * Bool))");
    assert_eq!(ty, "Nat");
}

#[test]
fn let_binding_and_ascription() {
    let (ty, nf) = infer_and_print("let x : Nat = 3 in succ x");
    assert_eq!(ty, "Nat");
    assert_eq!(nf, "succ (succ (succ (succ zero)))");
}

#[test]
fn parse_and_lower_helper_and_rejects_bad_input() {
    let term = parse_and_lower("\\x : Bool => x").unwrap();
    assert!(matches!(&*term, tpt_rustir_core::TermKind::Lambda(_, _)));
    assert!(parse_and_lower("=>").is_err());
}
