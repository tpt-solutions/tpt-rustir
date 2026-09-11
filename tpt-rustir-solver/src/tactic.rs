//! Tactic Engine DSL: composes e-graph saturation (and, behind the `smt`
//! feature, SMT) into named tactics that discharge proof goals.

use egg::{Pattern, RecExpr, Rewrite};

use crate::lang::{algebraic_rules, definitional_rules, Expr, Rw};
use crate::simplify::{parse, prove_equal, ParseError};

/// A proof obligation: `lhs` and `rhs` should be provably equal.
#[derive(Debug, Clone)]
pub struct Goal {
    pub lhs: String,
    pub rhs: String,
}

impl Goal {
    pub fn new(lhs: impl Into<String>, rhs: impl Into<String>) -> Self {
        Goal {
            lhs: lhs.into(),
            rhs: rhs.into(),
        }
    }

    /// A tautology goal, phrased as `expr` being equal to `true`.
    pub fn tautology(expr: impl Into<String>) -> Self {
        Goal::new(expr, "true")
    }
}

#[derive(Debug, Clone)]
pub enum Tactic {
    /// Run equality saturation with the full algebraic/logical rule set.
    Simp,
    /// Try `Simp`; if that fails and the goal has a designated induction
    /// variable, fall back to `Induction`; if that fails too, fall back to
    /// the SMT solver (behind the `smt` feature).
    Auto { induction_var: Option<String> },
    /// Structural induction over `Nat` on `var`, discharging the base and
    /// step subgoals with `Simp`.
    Induction { var: String },
    /// Hand the goal to the SMT solver (behind the `smt` feature).
    Smt,
    /// Run tactics in order, succeeding as soon as one does.
    OrElse(Vec<Tactic>),
}

#[derive(Debug, Clone)]
pub enum TacticError {
    Parse(ParseError),
    SimpFailed,
    BaseCaseFailed,
    StepCaseFailed,
    NoTacticApplied,
    /// `Tactic::Smt` (or the SMT leg of `Auto`) was used without building
    /// with the `smt` feature.
    SmtUnavailable,
    /// The SMT solver ran but could not discharge the goal.
    #[cfg(feature = "smt")]
    Smt(crate::smt::SmtError),
}

impl std::fmt::Display for TacticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TacticError::Parse(e) => write!(f, "{e}"),
            TacticError::SimpFailed => {
                write!(f, "simp: goal is not provable by equality saturation alone")
            }
            TacticError::BaseCaseFailed => write!(f, "induction: base case unprovable"),
            TacticError::StepCaseFailed => write!(f, "induction: step case unprovable"),
            TacticError::NoTacticApplied => {
                write!(f, "no tactic in the alternatives discharged the goal")
            }
            TacticError::SmtUnavailable => write!(
                f,
                "smt: rebuild with the `smt` feature (requires a system Z3)"
            ),
            #[cfg(feature = "smt")]
            TacticError::Smt(e) => write!(f, "smt: {e}"),
        }
    }
}
impl std::error::Error for TacticError {}
impl From<ParseError> for TacticError {
    fn from(e: ParseError) -> Self {
        TacticError::Parse(e)
    }
}

/// Split `template` on whitespace/parens and replace tokens exactly equal to
/// `var` with `replacement`, leaving every other token (including any that
/// merely *contain* `var` as a substring, e.g. `not` vs. `n`) untouched.
fn substitute_var(template: &str, var: &str, replacement: &str) -> String {
    let mut out = String::new();
    let mut cur = String::new();
    for ch in template.chars() {
        if ch == '(' || ch == ')' {
            push_token(&mut out, &mut cur, var, replacement);
            out.push(ch);
        } else if ch.is_whitespace() {
            push_token(&mut out, &mut cur, var, replacement);
        } else {
            cur.push(ch);
        }
    }
    push_token(&mut out, &mut cur, var, replacement);
    out
}

fn push_token(out: &mut String, cur: &mut String, var: &str, replacement: &str) {
    if cur.is_empty() {
        return;
    }
    if cur == var {
        out.push_str(replacement);
    } else {
        out.push_str(cur);
    }
    out.push(' ');
    cur.clear();
}

fn run_simp(goal: &Goal) -> Result<(), TacticError> {
    run_with_rules(goal, &algebraic_rules())
}

/// Try to discharge `goal` with an arbitrary rule set (exposed for tests and
/// callers that want finer control than the named tactics give).
pub fn run_with_rules(goal: &Goal, rules: &[Rw]) -> Result<(), TacticError> {
    let lhs = parse(&goal.lhs)?;
    let rhs = parse(&goal.rhs)?;
    if prove_equal(&lhs, &rhs, rules) {
        Ok(())
    } else {
        Err(TacticError::SimpFailed)
    }
}

fn run_induction(goal: &Goal, var: &str) -> Result<(), TacticError> {
    // Deliberately uses only the *definitional* recursion rules for +/*, not
    // the comm/assoc "given" axioms in `algebraic_rules` — those would make
    // some goals trivially provable without induction ever contributing
    // anything, defeating the point of this tactic.
    let base_lhs = substitute_var(&goal.lhs, var, "zero");
    let base_rhs = substitute_var(&goal.rhs, var, "zero");
    run_with_rules(&Goal::new(base_lhs, base_rhs), &definitional_rules())
        .map_err(|_| TacticError::BaseCaseFailed)?;

    // Step case: var := (succ var), with the induction hypothesis
    // `lhs[var] = rhs[var]` added as an extra rewrite rule for this subgoal.
    let ih_lhs_pattern: Pattern<Expr> = substitute_var(&goal.lhs, var, "?v").parse().unwrap();
    let ih_rhs_pattern: Pattern<Expr> = substitute_var(&goal.rhs, var, "?v").parse().unwrap();
    let ih_rule: Rw = Rewrite::new("induction-hypothesis", ih_lhs_pattern, ih_rhs_pattern).unwrap();
    let mut rules = definitional_rules();
    rules.push(ih_rule);

    let succ_var = format!("(succ {var})");
    let step_lhs = substitute_var(&goal.lhs, var, &succ_var);
    let step_rhs = substitute_var(&goal.rhs, var, &succ_var);
    run_with_rules(&Goal::new(step_lhs, step_rhs), &rules).map_err(|_| TacticError::StepCaseFailed)
}

/// Try to discharge `goal` with the SMT fallback. Without the `smt` feature
/// this reports `TacticError::SmtUnavailable`.
#[cfg(feature = "smt")]
fn run_smt(goal: &Goal) -> Result<(), TacticError> {
    crate::smt::smt_prove(goal).map_err(TacticError::Smt)
}

/// Try to discharge `goal` with the SMT fallback. Without the `smt` feature
/// this reports `TacticError::SmtUnavailable`.
#[cfg(not(feature = "smt"))]
fn run_smt(_goal: &Goal) -> Result<(), TacticError> {
    Err(TacticError::SmtUnavailable)
}

pub fn run(tactic: &Tactic, goal: &Goal) -> Result<(), TacticError> {
    match tactic {
        Tactic::Simp => run_simp(goal),
        Tactic::Induction { var } => run_induction(goal, var),
        Tactic::Smt => run_smt(goal),
        Tactic::Auto { induction_var } => {
            // Try direct unfolding first (definitional rules only — not the
            // comm/assoc "given" lemmas `Simp` uses, since those would make
            // this indistinguishable from `Simp` and never actually reach
            // the fallbacks).
            if run_with_rules(goal, &definitional_rules()).is_ok() {
                return Ok(());
            }
            if let Some(var) = induction_var {
                if run_induction(goal, var).is_ok() {
                    return Ok(());
                }
            }
            // Last resort: hand the goal to the SMT solver. This is where
            // genuine arithmetic obligations (e.g. `x <= x + 0` shapes beyond
            // the e-graph rules) get discharged.
            run_smt(goal)
        }
        Tactic::OrElse(tactics) => {
            for t in tactics {
                if run(t, goal).is_ok() {
                    return Ok(());
                }
            }
            Err(TacticError::NoTacticApplied)
        }
    }
}

/// Extract the normal form of a raw expression string (for display/tests).
pub fn normal_form(expr: &str) -> Result<RecExpr<Expr>, TacticError> {
    let e = parse(expr)?;
    Ok(crate::simplify::simplify(&e, &algebraic_rules()))
}
