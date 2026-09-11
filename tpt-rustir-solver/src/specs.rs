//! Spec verification pipeline: verify tpt specification strings against the
//! kernel and the tactic engine.
//!
//! This is the shared entry point used by `#[tpt::spec]` / `#[tpt::verify]`
//! (proc-macro crates may only export proc-macro items, so the workhorse
//! lives here) and by `cargo tpt verify` in the CLI.
//!
//! Spec strings come in two forms:
//! 1. solver obligations: `<lhs> = <rhs>` with e-graph `Expr`-language sides,
//!    split at the top-level ` = ` and discharged by the tactic engine
//!    (e-graph saturation, then the SMT fallback behind the `smt` feature);
//! 2. anything else: a tpt term, elaborated via `tpt-rustir-syntax` and
//!    kernel-type-checked.

use crate::tactic::{run, Goal, Tactic};

/// The discharge pipeline for solver obligations: e-graph saturation first,
/// then the SMT solver (when built with the `smt` feature).
pub fn default_goal_tactic() -> Tactic {
    Tactic::OrElse(vec![Tactic::Simp, Tactic::Smt])
}

/// Split a spec of the form `<lhs> = <rhs>` at the top-level ` = `
/// (outside any parentheses/brackets), if present.
pub fn split_top_level_eq(spec: &str) -> Option<(String, String)> {
    let chars: Vec<char> = spec.chars().collect();
    let mut depth = 0i32;
    for i in 0..chars.len() {
        match chars[i] {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '=' if depth == 0 => {
                // Require whitespace padding so `=` fragments that are part
                // of symbols don't split.
                let before = i.checked_sub(1).map(|j| chars[j]);
                let after = chars.get(i + 1).copied();
                if before.is_some_and(char::is_whitespace)
                    && after.is_some_and(char::is_whitespace)
                {
                    let lhs: String = chars[..i].iter().collect();
                    let rhs: String = chars[i + 1..].iter().collect();
                    return Some((lhs.trim().to_string(), rhs.trim().to_string()));
                }
            }
            _ => {}
        }
    }
    None
}

/// Check a spec string without discharging any proof obligation.
///
/// 1. `<lhs> = <rhs>` — confirms both sides parse as solver expressions.
/// 2. anything else — a tpt term, elaborated and kernel-type-checked (kernel
///    type-checking is not a proof obligation to discharge, so this is the
///    same work `verify_spec_string` does for this case).
pub fn check_spec_string(spec: &str) -> Result<(), String> {
    if let Some((lhs, rhs)) = split_top_level_eq(spec) {
        crate::simplify::parse(&lhs)
            .map_err(|e| format!("spec `{spec}` failed to parse: {e}"))?;
        crate::simplify::parse(&rhs)
            .map_err(|e| format!("spec `{spec}` failed to parse: {e}"))?;
        Ok(())
    } else {
        let term = tpt_rustir_syntax::parse_and_lower(spec)
            .map_err(|e| format!("spec `{spec}` failed to elaborate: {e}"))?;
        let env = tpt_rustir_core::GlobalEnv::new();
        tpt_rustir_core::infer(&env, &Vec::new(), &term)
            .map(|_| ())
            .map_err(|e| format!("spec `{spec}` is ill-typed: {e}"))
    }
}

/// Verify a spec string.
///
/// 1. `<lhs> = <rhs>` — a solver obligation, discharged by the tactic engine.
/// 2. anything else — a tpt term, elaborated and kernel-type-checked.
pub fn verify_spec_string(spec: &str) -> Result<(), String> {
    if let Some((lhs, rhs)) = split_top_level_eq(spec) {
        let goal = Goal::new(lhs, rhs);
        run(&default_goal_tactic(), &goal)
            .map_err(|e| format!("spec obligation `{spec}` could not be discharged: {e}"))
    } else {
        let term = tpt_rustir_syntax::parse_and_lower(spec)
            .map_err(|e| format!("spec `{spec}` failed to elaborate: {e}"))?;
        let env = tpt_rustir_core::GlobalEnv::new();
        tpt_rustir_core::infer(&env, &Vec::new(), &term)
            .map(|_| ())
            .map_err(|e| format!("spec `{spec}` is ill-typed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_tpt_spec_verifies() {
        assert!(verify_spec_string("\\x : Nat => succ x").is_ok());
        assert!(verify_spec_string("(\\x : Nat => succ x) 2").is_ok());
    }

    #[test]
    fn bogus_tpt_spec_is_rejected() {
        // `x > 0` is not tpt: `>` is not a valid application head.
        assert!(verify_spec_string("\\x : Nat => x > 0").is_err());
        // A Bool applied to an argument is ill-typed.
        assert!(verify_spec_string("true zero").is_err());
    }

    #[test]
    fn solver_obligation_spec_discharges() {
        assert!(verify_spec_string("(+ (+ a b) c) = (+ c (+ b a))").is_ok());
        assert!(verify_spec_string("(+ n zero) = zero").is_err());
    }

    #[test]
    fn check_accepts_well_formed_obligation_without_discharging_it() {
        // Unlike `verify_spec_string`, `check_spec_string` only parses the
        // two sides — it doesn't require the equality to actually hold.
        assert!(check_spec_string("(+ n zero) = zero").is_ok());
        assert!(check_spec_string("(+ (+ a b) c) = (+ c (+ b a))").is_ok());
    }

    #[test]
    fn check_rejects_malformed_spec() {
        assert!(check_spec_string("(+ n zero =").is_err());
        assert!(check_spec_string("\\x : Nat => x > 0").is_err());
    }

    #[test]
    fn top_level_eq_splits_outside_parens() {
        assert_eq!(
            split_top_level_eq("(and a b) = true"),
            Some(("(and a b)".to_string(), "true".to_string()))
        );
        assert_eq!(split_top_level_eq("(x = y)"), None);
        assert_eq!(split_top_level_eq("\\x : Nat => x"), None);
    }
}