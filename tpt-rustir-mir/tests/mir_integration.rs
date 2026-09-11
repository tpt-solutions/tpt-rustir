//! Integration tests against sample MIR-lite bodies: kernel-checked
//! assignments, unsafe-block justification, tactic-discharged assertion
//! obligations, and extraction back into Rust source.

use tpt_rustir_core::term;
use tpt_rustir_mir::extract::{
    body_to_rust_source, pair_type_to_struct_source, pair_type_to_tuple_source,
    pair_type_to_tuple_source_with_witness, pair_value_to_rust, pi_type_to_fn_source,
};
use tpt_rustir_mir::mir::{
    BasicBlock, BasicBlockId, BinOp, Local, LocalId, MirBody, Operand, Place, Rvalue, Statement,
    Terminator,
};
use tpt_rustir_mir::stdlib;
use tpt_rustir_mir::verify::{verify_body, ObligationStatus, VerifyError};
use tpt_rustir_mir::RustType;

fn u32_ty() -> RustType {
    RustType::Primitive("u32".to_string())
}

/// `fn bounded(i: u32) -> u32 { let chk = i <= i; assert(chk); _0 = i }`
fn bounded_body() -> MirBody {
    MirBody {
        args: vec![LocalId(1)],
        locals: vec![
            Local::new("_0", u32_ty()),
            Local::new("i", u32_ty()),
            Local::new("chk", RustType::Primitive("bool".to_string())),
        ],
        blocks: vec![
            BasicBlock {
                statements: vec![Statement::Assign {
                    place: Place(LocalId(2)),
                    rvalue: Rvalue::BinaryOp(
                        BinOp::Le,
                        Operand::Local(LocalId(1)),
                        Operand::Local(LocalId(1)),
                    ),
                }],
                terminator: Terminator::Assert {
                    condition: Operand::Local(LocalId(2)),
                    expected: true,
                    target: BasicBlockId(1),
                },
            },
            BasicBlock {
                statements: vec![Statement::Assign {
                    place: Place(LocalId(0)),
                    rvalue: Rvalue::Use(Operand::Local(LocalId(1))),
                }],
                terminator: Terminator::Return,
            },
        ],
    }
}

#[test]
fn well_typed_body_verifies_with_discharged_assert() {
    let body = bounded_body();
    let report = verify_body(&body).expect("bounded body should verify");
    assert!(
        report.all_discharged(),
        "the `i <= i` assert obligation should be discharged, got {:?}",
        report.outcomes
    );
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(
        report.outcomes[0].status,
        ObligationStatus::Discharged { by: "auto".to_string() }
    );
}

#[test]
fn ill_typed_assignment_is_rejected() {
    let mut body = bounded_body();
    // Assign a Nat expression to the `bool`-typed local `chk`.
    body.blocks[0].statements[0] = Statement::Assign {
        place: Place(LocalId(2)),
        rvalue: Rvalue::BinaryOp(
            BinOp::Add,
            Operand::Local(LocalId(1)),
            Operand::Local(LocalId(1)),
        ),
    };
    let err = verify_body(&body).expect_err("bool local must not receive a Nat value");
    assert!(matches!(err, VerifyError::IllTypedAssignment { .. }), "{err}");
}

#[test]
fn unprovable_assert_is_reported() {
    let mut body = bounded_body();
    // `let s = i + 1; let chk = s <= i;` — false for every i; neither the
    // e-graph rules nor (feature-off) SMT can discharge it.
    body.blocks[0].statements = vec![
        Statement::Assign {
            place: Place(LocalId(2)),
            rvalue: Rvalue::BinaryOp(BinOp::Le, Operand::Local(LocalId(3)), Operand::Local(LocalId(1))),
        },
    ];
    body.locals.push(Local::new("s", u32_ty())); // local 3
    body.blocks[0].statements.insert(
        0,
        Statement::Assign {
            place: Place(LocalId(3)),
            rvalue: Rvalue::BinaryOp(BinOp::Add, Operand::Local(LocalId(1)), Operand::NatConst(1)),
        },
    );
    let err = verify_body(&body).expect_err("(i + 1) <= i must not be provable");
    assert!(matches!(err, VerifyError::ObligationFailed { .. }), "{err}");
}

// ---------------------------------------------------------------------------
// Unsafe-block justification
// ---------------------------------------------------------------------------

/// Body with one `unsafe` block parameterized by its justification parts.
fn unsafe_body(
    obligation: tpt_rustir_core::Term,
    assumption: Option<tpt_rustir_core::Term>,
    proof: Option<tpt_rustir_core::Term>,
) -> MirBody {
    MirBody {
        args: vec![LocalId(1)],
        locals: vec![Local::new("_0", u32_ty()), Local::new("i", u32_ty())],
        blocks: vec![BasicBlock {
            statements: vec![Statement::Unsafe {
                description: "raw pointer read".to_string(),
                obligation,
                assumption,
                proof,
                statements: vec![],
            }],
            terminator: Terminator::Return,
        }],
    }
}

#[test]
fn justified_unsafe_block_is_accepted() {
    // obligation: `i <= i`; the block records the same proposition as an
    // (opaque, recorded) assumption and its proof is that hypothesis.
    let le_ii = term::app(
        term::app(term::const_("le"), term::var(0)),
        term::var(0),
    );
    let body = unsafe_body(le_ii.clone(), Some(le_ii), Some(term::var(0)));
    let report = verify_body(&body).expect("assumption-backed unsafe block should verify");
    assert_eq!(
        report.outcomes[0].status,
        ObligationStatus::Discharged { by: "kernel".to_string() }
    );
}

#[test]
fn automatically_dischargeable_unsafe_obligation_is_accepted() {
    // obligation: `i <= i` with no proof attached — the tactic engine
    // discharges it (le-refl).
    let le_ii = term::app(
        term::app(term::const_("le"), term::var(0)),
        term::var(0),
    );
    let body = unsafe_body(le_ii, None, None);
    let report = verify_body(&body).expect("auto-dischargeable unsafe block should verify");
    assert_eq!(
        report.outcomes[0].status,
        ObligationStatus::Discharged { by: "auto".to_string() }
    );
}

#[test]
fn unjustified_unsafe_block_is_rejected() {
    // obligation: `i + 1 <= i` — false, no proof, no assumption.
    let obligation = term::app(
        term::app(
            term::const_("le"),
            term::app(
                term::app(term::const_("add"), term::var(0)),
                term::succ(term::zero()),
            ),
        ),
        term::var(0),
    );
    let body = unsafe_body(obligation, None, None);
    let err = verify_body(&body).expect_err("false unsafe obligation must be rejected");
    assert!(matches!(err, VerifyError::ObligationFailed { .. }), "{err}");
}

#[test]
fn wrong_proof_is_rejected() {
    // obligation: `i <= i`, assumption present, but the "proof" is the Nat
    // variable itself, which does not have the obligation as its type.
    let le_ii = term::app(
        term::app(term::const_("le"), term::var(0)),
        term::var(0),
    );
    let body = unsafe_body(le_ii.clone(), Some(le_ii), Some(term::var(1)));
    let err = verify_body(&body).expect_err("a Nat variable is not a proof of a Bool prop");
    assert!(
        matches!(err, VerifyError::UnsafeNotJustified { .. }),
        "{err}"
    );
}

// ---------------------------------------------------------------------------
// Extraction
// ---------------------------------------------------------------------------

#[test]
fn pair_type_extracts_to_tuple() {
    let env = stdlib::stdlib();
    let sigma = term::sigma(term::nat(), term::bool_ty());
    let t = pair_type_to_tuple_source(&env, &sigma).expect("non-dependent pair extracts");
    assert_eq!(t.source, "(u32, bool)");
    assert!(!t.dependency_resolved);
}

#[test]
fn triple_pair_type_extracts_recursively() {
    let env = stdlib::stdlib();
    let sigma = term::sigma(term::nat(), term::sigma(term::bool_ty(), term::nat()));
    let t = pair_type_to_tuple_source(&env, &sigma).unwrap();
    assert_eq!(t.source, "(u32, bool, u32)");
}

#[test]
fn dependent_pair_requires_witness() {
    let env = stdlib::stdlib();
    // Sigma(x : Nat, T x) where T = \n : Nat. natrec(\_ . Type0, Natu32,
    // \n'.\h. Inti32, n) — a genuinely value-dependent type family: it is
    // stuck on the symbolic binder, so extraction without a witness fails.
    let t_of = |n: tpt_rustir_core::Term| {
        term::nat_rec(
            term::lambda(term::nat(), term::universe(0)),
            term::const_("Natu32"),
            term::lambda(term::nat(), term::lambda(term::nat(), term::const_("Inti32"))),
            n,
        )
    };
    let dependent = term::sigma(term::nat(), t_of(term::var(0)));
    assert!(matches!(
        pair_type_to_tuple_source(&env, &dependent),
        Err(tpt_rustir_mir::extract::ExtractError::DependencyNotResolvable { .. })
    ));
    // With a witness for the first component the dependency resolves
    // (natrec on zero → the base case Natu32).
    let t = pair_type_to_tuple_source_with_witness(&env, &dependent, Some(&term::zero()))
        .expect("witness resolves the dependency");
    assert_eq!(t.source, "(u32, u32)");
    assert!(t.dependency_resolved);
}

#[test]
fn pair_type_extracts_to_struct() {
    let env = stdlib::stdlib();
    let sigma = term::sigma(term::nat(), term::bool_ty());
    let src = pair_type_to_struct_source(&env, "Bounded", &sigma).unwrap();
    assert!(src.contains("pub struct Bounded"), "{src}");
    assert!(src.contains("pub field0: u32"), "{src}");
    assert!(src.contains("pub field1: bool"), "{src}");
}

#[test]
fn pair_value_extracts_to_tuple_expr() {
    let p = term::pair(
        term::zero(),
        term::succ(term::zero()),
        term::sigma(term::nat(), term::nat()),
    );
    assert_eq!(pair_value_to_rust(&p).unwrap(), "(0, 1)");
}

#[test]
fn pi_type_extracts_to_fn_signature() {
    let env = stdlib::stdlib();
    let ty = term::pi(term::nat(), term::nat());
    assert_eq!(pi_type_to_fn_source(&env, &ty).unwrap(), "fn(u32) -> u32");

    let two_arg = term::pi(term::nat(), term::pi(term::bool_ty(), term::nat()));
    assert_eq!(
        pi_type_to_fn_source(&env, &two_arg).unwrap(),
        "fn(u32, bool) -> u32"
    );
}

#[test]
fn body_extracts_to_rust_source() {
    let src = body_to_rust_source("bounded", &bounded_body()).unwrap();
    assert!(src.contains("pub fn bounded(i: u32) -> u32"), "{src}");
    assert!(src.contains("let chk = (i <= i);"), "{src}");
    assert!(src.contains("debug_assert_eq!(chk, true);"), "{src}");
    assert!(src.ends_with("    _0\n}\n"), "{src}");
}