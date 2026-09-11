use tpt_rustir_solver::{definitional_rules, parse, run, run_with_rules, tactic::Tactic, Goal};

#[test]
fn simp_discharges_algebraic_equalities() {
    // Associativity/commutativity together with the recursive definitions.
    let goal = Goal::new("(+ (+ a b) c)", "(+ c (+ b a))");
    run(&Tactic::Simp, &goal).expect("simp should prove this algebraic identity");

    // Peano-style computation: 1 + 1 = 2.
    let goal = Goal::new("(+ (succ zero) (succ zero))", "(succ (succ zero))");
    run(&Tactic::Simp, &goal).expect("simp should compute 1 + 1 = 2");

    // Distribution-free multiplication identity: 2 * 0 = 0.
    let goal = Goal::new("(* (succ (succ zero)) zero)", "zero");
    run(&Tactic::Simp, &goal).expect("simp should prove n * 0 = 0");
}

#[test]
fn simp_discharges_logical_implications() {
    // A -> A is a tautology.
    let goal = Goal::tautology("(implies A A)");
    run(&Tactic::Simp, &goal).expect("simp should prove A -> A");

    // (A and B) -> A is a tautology.
    let goal = Goal::tautology("(implies (and A B) A)");
    run(&Tactic::Simp, &goal).expect("simp should prove (A and B) -> A");

    // Excluded middle itself.
    let goal = Goal::tautology("(or A (not A))");
    run(&Tactic::Simp, &goal).expect("simp should prove excluded middle");
}

#[test]
fn plain_definitional_rules_cannot_prove_right_identity_without_induction() {
    // Without commutativity, `n + zero = n` isn't reachable from the purely
    // recursive definitions when `n` is an opaque symbol — this is the
    // textbook example that requires induction.
    let goal = Goal::new("(+ n zero)", "n");
    let result = run_with_rules(&goal, &definitional_rules());
    assert!(
        result.is_err(),
        "definitional rules alone should get stuck, not prove this"
    );
}

#[test]
fn induction_proves_right_identity() {
    let goal = Goal::new("(+ n zero)", "n");
    run(
        &Tactic::Induction {
            var: "n".to_string(),
        },
        &goal,
    )
    .expect("induction should prove the Peano right-identity theorem");
}

#[test]
fn auto_falls_back_to_induction() {
    let goal = Goal::new("(+ n zero)", "n");
    run(
        &Tactic::Auto {
            induction_var: Some("n".to_string()),
        },
        &goal,
    )
    .expect("auto should fall back to induction when simp alone is insufficient");
}

#[test]
fn parse_error_is_reported() {
    assert!(parse("(+ a").is_err());
}

#[test]
fn le_rules_are_sound_and_terminating() {
    // (le zero x) is true for every x.
    let goal = Goal::new("(le zero x)", "true");
    run(&Tactic::Simp, &goal).expect("le zero x holds by the definitional rule");

    // (le (succ x) zero) is false.
    let goal = Goal::new("(le (succ x) zero)", "false");
    run(&Tactic::Simp, &goal).expect("le (succ x) zero fails by the definitional rule");

    // (le x x) holds by reflexivity.
    let goal = Goal::new("(le x x)", "true");
    run(&Tactic::Simp, &goal).expect("le is reflexive");

    // (le x (succ x)) is true over Nat but out of reach for the current
    // e-graph rules (x is symbolic) — this is exactly the class of goals the
    // SMT fallback exists for. Assert simp gives up instead of looping.
    let goal = Goal::new("(le x (succ x))", "true");
    assert!(
        run(&Tactic::Simp, &goal).is_err(),
        "e-graph rules should not prove x <= succ x (that needs SMT)"
    );
}

// ---------------------------------------------------------------------------
// SMT fallback tests. Compiled only with `--features smt`, which requires a
// system Z3 installation; CI does not exercise them.
// ---------------------------------------------------------------------------
#[cfg(feature = "smt")]
mod smt_tests {
    use super::*;
    use tpt_rustir_solver::smt::{smt_available, smt_prove};

    #[test]
    fn z3_initializes() {
        assert!(smt_available());
    }

    #[test]
    fn smt_proves_linear_arithmetic() {
        // (x + 0 <= x) for every x — beyond the e-graph's le rules.
        let goal = Goal::new("(le (+ x zero) x)", "true");
        smt_prove(&goal).expect("z3 should prove x + 0 <= x");
    }

    #[test]
    fn smt_proves_integer_identities() {
        let goal = Goal::new("(+ x (* 2 y))", "(+ x (+ y y))");
        smt_prove(&goal).expect("z3 should prove x + 2y = x + y + y");
    }

    #[test]
    fn smt_rejects_false_goals() {
        let goal = Goal::new("(le (succ x) x)", "true");
        assert!(smt_prove(&goal).is_err());
    }
}
