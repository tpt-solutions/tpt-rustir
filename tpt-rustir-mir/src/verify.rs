//! Verification of MIR-lite bodies against `tpt-rustir-core`.
//!
//! Pipeline:
//! 1. walk reachable blocks, mapping every assigned expression into a core
//!    term (assignments are *inlined*: a local's value is its defining
//!    expression; this models straight-line code without loops — the
//!    `stable_mir` adapter will lift this to a proper let-chain);
//! 2. kernel-check every assignment against the target local's declared
//!    type;
//! 3. kernel-check every `unsafe` block's justification: its obligation must
//!    be a well-formed `Bool` proposition, and the proof (if given) must have
//!    the obligation as its type. A proof that references no assumption must
//!    additionally *compute* to `true`; a proof may instead rely on the
//!    block's recorded `assumption` (an opaque, trusted hypothesis);
//! 4. discharge every `Assert`'s proof obligation with the tactic engine
//!    (e-graph saturation, then the SMT fallback behind the `smt` feature);
//!    obligations outside the translatable arithmetic subset are reported as
//!    `Unchecked`, not failures.

use std::collections::{HashMap, HashSet};

use tpt_rustir_core::term::{self, Term};
use tpt_rustir_core::{check, conv, infer, normalize, Ctx, GlobalEnv};
use tpt_rustir_solver::tactic::{run, Goal, Tactic};

use crate::mir::{BasicBlockId, LocalId, MirBody, Operand, Rvalue, Statement, Terminator};
use crate::stdlib;

/// The default discharge pipeline for proof obligations: e-graph equality
/// saturation first, then the SMT solver (when built with the `smt` feature).
pub fn default_tactic() -> Tactic {
    Tactic::OrElse(vec![Tactic::Simp, Tactic::Smt])
}

#[derive(Debug, Clone)]
pub enum VerifyError {
    /// A statement reads a local that has no value yet.
    UnassignedLocal { local: u32 },
    /// An assignment's expression doesn't have the target local's type.
    IllTypedAssignment {
        local: String,
        error: tpt_rustir_core::check::TypeError,
    },
    /// An `unsafe` block's obligation is not a well-formed Bool proposition.
    IllFormedObligation {
        description: String,
        error: tpt_rustir_core::check::TypeError,
    },
    /// An `unsafe` block's recorded assumption is not a Bool proposition.
    IllFormedAssumption {
        description: String,
        error: tpt_rustir_core::check::TypeError,
    },
    /// An `unsafe` block was not justified: no proof, or a proof that failed
    /// to type-check / compute to `true`.
    UnsafeNotJustified { description: String, reason: String },
    /// A solver obligation could not be discharged.
    ObligationFailed {
        description: String,
        reason: String,
    },
    /// A terminator targets a block that does not exist.
    MissingBlock(u32),
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerifyError::UnassignedLocal { local } => {
                write!(f, "read of unassigned local _{local}")
            }
            VerifyError::IllTypedAssignment { local, error } => {
                write!(f, "assignment to `{local}` is ill-typed: {error}")
            }
            VerifyError::IllFormedObligation { description, error } => write!(
                f,
                "unsafe block `{description}`: obligation is not a Bool proposition: {error}"
            ),
            VerifyError::IllFormedAssumption { description, error } => write!(
                f,
                "unsafe block `{description}`: assumption is not a Bool proposition: {error}"
            ),
            VerifyError::UnsafeNotJustified { description, reason } => {
                write!(f, "unsafe block `{description}` is not justified: {reason}")
            }
            VerifyError::ObligationFailed { description, reason } => {
                write!(f, "obligation `{description}` could not be discharged: {reason}")
            }
            VerifyError::MissingBlock(id) => write!(f, "terminator targets missing block b{id}"),
        }
    }
}
impl std::error::Error for VerifyError {}

/// Outcome of one discharged obligation.
#[derive(Debug, Clone)]
pub struct ObligationOutcome {
    pub description: String,
    pub status: ObligationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObligationStatus {
    /// Proved; `by` names the tactic that discharged it (or the kernel).
    Discharged { by: String },
    /// Outside the automatically checkable subset; recorded, not failed.
    Unchecked { reason: String },
}

/// The result of verifying a body.
#[derive(Debug, Clone, Default)]
pub struct VerificationReport {
    pub outcomes: Vec<ObligationOutcome>,
}

impl VerificationReport {
    pub fn all_discharged(&self) -> bool {
        self.outcomes
            .iter()
            .all(|o| matches!(o.status, ObligationStatus::Discharged { .. }))
    }
}

/// The de Bruijn index of argument `k` (0-based, declaration order) in a
/// context holding all argument types.
fn arg_var(k: usize, n_args: usize) -> Term {
    term::var(n_args - 1 - k)
}

/// Build the core term for an operand given the inlined value map.
fn operand_term(op: &Operand, values: &HashMap<LocalId, Term>) -> Result<Term, VerifyError> {
    Ok(match op {
        Operand::NatConst(n) => stdlib::num(*n),
        Operand::BoolConst(b) => {
            if *b {
                term::true_()
            } else {
                term::false_()
            }
        }
        Operand::Local(id) => values
            .get(id)
            .cloned()
            .ok_or(VerifyError::UnassignedLocal { local: id.0 })?,
    })
}

/// Build the core term for an rvalue.
fn rvalue_term(rv: &Rvalue, values: &HashMap<LocalId, Term>) -> Result<Term, VerifyError> {
    match rv {
        Rvalue::Use(op) => operand_term(op, values),
        Rvalue::BinaryOp(binop, a, b) => {
            let ta = operand_term(a, values)?;
            let tb = operand_term(b, values)?;
            Ok(term::app(
                term::app(term::const_(binop.stdlib_const()), ta),
                tb,
            ))
        }
    }
}

fn push_ty(ctx: &Ctx, ty: Term) -> Ctx {
    let mut ctx = ctx.clone();
    ctx.push(ty);
    ctx
}

/// Kernel-check the justification of an `unsafe` block against its obligation.
///
/// A proof term must *have the obligation as its type*. When the proof relies
/// on the block's `assumption`, the assumption is pushed as an opaque
/// hypothesis (its own proposition) and the obligation is only type-checked —
/// the assumption is trusted and recorded. Without an assumption, the proof
/// must also normalize to `true`; otherwise any term (including the false
/// proposition itself) would reflexively "prove" itself.
fn check_unsafe_proof(
    env: &GlobalEnv,
    ctx: &Ctx,
    description: &str,
    obligation: &Term,
    assumption: Option<&Term>,
    proof: Option<&Term>,
) -> Result<(), VerifyError> {
    let proof = proof.ok_or_else(|| VerifyError::UnsafeNotJustified {
        description: description.to_string(),
        reason: "no proof term was attached".to_string(),
    })?;

    let (proof_ctx, obligation) = match assumption {
        Some(a) => {
            // The assumption must be a well-formed Bool proposition in the
            // surrounding context.
            let a_ty =
                infer(env, ctx, a).map_err(|e| VerifyError::IllFormedAssumption {
                    description: description.to_string(),
                    error: e,
                })?;
            if !conv(env, &a_ty, &term::bool_ty()) {
                return Err(VerifyError::IllFormedAssumption {
                    description: description.to_string(),
                    error: tpt_rustir_core::check::TypeError::Mismatch {
                        expected: term::bool_ty(),
                        found: a_ty,
                    },
                });
            }
            // Push the assumption as an opaque hypothesis. Context entries
            // are stored at their push depth (cf. the kernel's own `push`),
            // so the assumption is stored unshifted; the obligation, read
            // one binder deeper in the proof context, is shifted by one.
            let ctx2 = push_ty(ctx, a.clone());
            (ctx2, term::shift(obligation, 0, 1))
        }
        None => (ctx.clone(), obligation.clone()),
    };

    check::check(env, &proof_ctx, proof, &obligation).map_err(|e| {
        VerifyError::UnsafeNotJustified {
            description: description.to_string(),
            reason: format!("proof does not type-check against the obligation: {e}"),
        }
    })?;

    if assumption.is_none() {
        let nf = normalize(env, proof);
        if !conv(env, &nf, &term::true_()) {
            return Err(VerifyError::UnsafeNotJustified {
                description: description.to_string(),
                reason: "proof does not compute to `true`".to_string(),
            });
        }
    }
    Ok(())
}

/// Try to discharge a Bool obligation `lhs ≡ rhs` with the tactic engine.
fn discharge_bool(
    description: &str,
    lhs: &Term,
    rhs: &Term,
) -> Result<ObligationOutcome, VerifyError> {
    let (lhs_expr, rhs_expr) = match (tpt_rustir_solver::term_to_expr(lhs), tpt_rustir_solver::term_to_expr(rhs)) {
        (Some(l), Some(r)) => (l, r),
        _ => {
            return Ok(ObligationOutcome {
                description: description.to_string(),
                status: ObligationStatus::Unchecked {
                    reason: "obligation is outside the translatable arithmetic subset"
                        .to_string(),
                },
            })
        }
    };
    let goal = Goal {
        lhs: lhs_expr.to_string(),
        rhs: rhs_expr.to_string(),
    };
    match run(&default_tactic(), &goal) {
        Ok(()) => Ok(ObligationOutcome {
            description: description.to_string(),
            status: ObligationStatus::Discharged {
                by: "auto".to_string(),
            },
        }),
        Err(e) => Err(VerifyError::ObligationFailed {
            description: description.to_string(),
            reason: e.to_string(),
        }),
    }
}

/// The core type a local is declared at; undeclared locals default to kernel
/// `Nat` (the MIR-lite arithmetic subset).
fn declared_type_term(body: &MirBody, id: LocalId) -> Term {
    body.local_ty(id)
        .cloned()
        .unwrap_or(crate::RustType::Primitive("u32".to_string()))
        .to_core_term()
}

/// Verify a MIR-lite body: kernel-check assignments and `unsafe`
/// justifications, and discharge `Assert` obligations with the tactic engine.
pub fn verify_body(body: &MirBody) -> Result<VerificationReport, VerifyError> {
    let env = crate::stdlib::stdlib();
    let mut report = VerificationReport::default();

    // Kernel context: argument types in declaration order.
    let n_args = body.args.len();
    let ctx: Ctx = body
        .args
        .iter()
        .map(|id| declared_type_term(body, *id))
        .collect();

    // Inlined value of every local so far (args are symbolic variables).
    let mut values: HashMap<LocalId, Term> = HashMap::new();
    for (k, id) in body.args.iter().enumerate() {
        values.insert(*id, arg_var(k, n_args));
    }

    // Depth-first walk over reachable blocks.
    let mut visited: HashSet<u32> = HashSet::new();
    let mut stack: Vec<BasicBlockId> = vec![BasicBlockId(0)];
    while let Some(bid) = stack.pop() {
        if !visited.insert(bid.0) {
            continue;
        }
        let block = body
            .blocks
            .get(bid.0 as usize)
            .ok_or(VerifyError::MissingBlock(bid.0))?;

        verify_statements(body, &env, &ctx, &block.statements, &mut values, &mut report)?;

        match &block.terminator {
            Terminator::Return => {}
            Terminator::Goto(target) => stack.push(*target),
            Terminator::Assert {
                condition,
                expected,
                target,
            } => {
                let cond_term = operand_term(condition, &values)?;
                // Well-formedness: the condition must be a Bool proposition.
                let cond_ty = infer(&env, &ctx, &cond_term).map_err(|e| {
                    VerifyError::IllFormedObligation {
                        description: format!("assert b{}", bid.0),
                        error: e,
                    }
                })?;
                if !conv(&env, &cond_ty, &term::bool_ty()) {
                    return Err(VerifyError::IllFormedObligation {
                        description: format!("assert b{}", bid.0),
                        error: tpt_rustir_core::check::TypeError::Mismatch {
                            expected: term::bool_ty(),
                            found: cond_ty,
                        },
                    });
                }
                let expected_term = if *expected {
                    term::true_()
                } else {
                    term::false_()
                };
                let outcome = discharge_bool(
                    &format!("assert b{}: condition holds", bid.0),
                    &cond_term,
                    &expected_term,
                )?;
                report.outcomes.push(outcome);
                stack.push(*target);
            }
        }
    }

    Ok(report)
}

/// Verify a statement list in place, updating the inlined value map.
fn verify_statements(
    body: &MirBody,
    env: &GlobalEnv,
    ctx: &Ctx,
    statements: &[Statement],
    values: &mut HashMap<LocalId, Term>,
    report: &mut VerificationReport,
) -> Result<(), VerifyError> {
    for statement in statements {
        match statement {
            Statement::Assign { place, rvalue } => {
                let rhs_term = rvalue_term(rvalue, values)?;
                let local = place.0;
                // The assigned expression must have the target local's
                // declared type (kernel-checked via conversion).
                let found_ty = infer(env, ctx, &rhs_term).map_err(|e| {
                    VerifyError::IllTypedAssignment {
                        local: format!("_{}", local.0),
                        error: e,
                    }
                })?;
                let declared_ty = declared_type_term(body, local);
                if !conv(env, &found_ty, &declared_ty) {
                    return Err(VerifyError::IllTypedAssignment {
                        local: format!("_{}", local.0),
                        error: tpt_rustir_core::check::TypeError::Mismatch {
                            expected: declared_ty,
                            found: found_ty,
                        },
                    });
                }
                values.insert(local, rhs_term);
            }
            Statement::Unsafe {
                description,
                obligation,
                assumption,
                proof,
                statements,
            } => {
                // 1. The obligation must be a well-formed Bool proposition.
                let ob_ty = infer(env, ctx, obligation).map_err(|e| {
                    VerifyError::IllFormedObligation {
                        description: description.clone(),
                        error: e,
                    }
                })?;
                if !conv(env, &ob_ty, &term::bool_ty()) {
                    return Err(VerifyError::IllFormedObligation {
                        description: description.clone(),
                        error: tpt_rustir_core::check::TypeError::Mismatch {
                            expected: term::bool_ty(),
                            found: ob_ty,
                        },
                    });
                }
                // 2. The block must be justified: either by a kernel-checked
                //    proof term, or by the tactic engine discharging the
                //    obligation.
                let status = match proof {
                    Some(_) => {
                        check_unsafe_proof(
                            env,
                            ctx,
                            description,
                            obligation,
                            assumption.as_ref(),
                            proof.as_ref(),
                        )?;
                        ObligationStatus::Discharged {
                            by: "kernel".to_string(),
                        }
                    }
                    None => {
                        // No proof attached: attempt automatic discharge.
                        discharge_bool(
                            &format!("unsafe `{description}`: safety obligation"),
                            obligation,
                            &term::true_(),
                        )?
                        .status
                    }
                };
                report.outcomes.push(ObligationOutcome {
                    description: description.clone(),
                    status,
                });
                // 3. Verify the block's inner statements in the same scope.
                verify_statements(body, env, ctx, statements, values, report)?;
            }
        }
    }
    Ok(())
}