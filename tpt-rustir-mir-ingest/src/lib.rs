//! Real `stable_mir` ingestion (the crate itself is now named `rustc_public`
//! — see its own docs) for `tpt-rustir-mir`.
//!
//! This is a deliberately separate, non-workspace crate rather than a module
//! of `tpt-rustir-mir` itself: it links against rustc's own internal crates
//! via `rustc_driver`, which requires a nightly toolchain with the
//! `rustc-dev`/`llvm-tools` components (see the repo root's
//! `rust-toolchain.toml`, which still applies here since it's directory-,
//! not workspace-, scoped) and is never publishable to crates.io. Keeping it
//! isolated means nothing in the main workspace — including its tests —
//! accidentally inherits that fragile linkage.
//!
//! Build/test this crate directly: `cargo test --manifest-path
//! tpt-rustir-mir-ingest/Cargo.toml`. Building it also needs
//! `RUSTFLAGS="-C prefer-dynamic"`: rustc_driver is itself a dynamically
//! linked build of the standard library, and without prefer-dynamic this
//! crate's own (statically linked) copy of std conflicts with it at link
//! time ("cannot satisfy dependencies so `std` only shows up once").

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_public;

use std::ops::ControlFlow;
use std::path::Path;

use rustc_public::mir::Mutability;
use rustc_public::ty::{IntTy, RigidTy, Ty, TyKind, UintTy};
use rustc_public::{CrateDef, ItemKind};

use tpt_rustir_mir::mir::{
    BasicBlock, BasicBlockId, BinOp, Local, LocalId, MirBody, Operand, Place, Rvalue, Statement,
    Terminator,
};
use tpt_rustir_mir::verify::{self, VerificationReport};
use tpt_rustir_mir::RustType;

/// One ingested function's signature, mapped into `RustType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnSignature {
    pub name: String,
    pub args: Vec<RustType>,
    pub ret: RustType,
}

/// Compile `path` as a standalone `lib` crate with the pinned nightly and
/// extract every local function's signature.
///
/// Emits metadata only (no codegen/linking is needed just to read bodies),
/// into `path`'s own directory as the out-dir — this also keeps concurrent
/// calls on differently-named files from racing on a shared output path.
pub fn ingest_file(path: &Path) -> Result<Vec<FnSignature>, String> {
    rustc_public::run!(&compiler_args(path)?, collect_signatures)
        .map_err(|e| format!("stable_mir ingestion of {}: {e:?}", path.display()))
}

/// Compile `path` as a standalone `lib` crate, extract every local
/// function's MIR body, translate it into `tpt_rustir_mir::mir::MirBody`,
/// and run it through the existing `tpt_rustir_mir::verify` pipeline —
/// the Milestone 3 end-to-end path: real Rust source → real `stable_mir` →
/// this crate's MIR-lite model → kernel/tactic-engine verification.
///
/// Bodies outside the translatable subset (see `translate_body`) are
/// reported per-function as `Err(reason)` rather than failing ingestion of
/// the whole file.
pub fn ingest_and_verify_file(
    path: &Path,
) -> Result<Vec<(String, Result<VerificationReport, String>)>, String> {
    rustc_public::run!(&compiler_args(path)?, collect_and_verify)
        .map_err(|e| format!("stable_mir ingestion of {}: {e:?}", path.display()))
}

/// The rustc invocation shared by `ingest_file` and `ingest_and_verify_file`.
///
/// `-C overflow-checks=off` keeps arithmetic MIR in the plain
/// `Rvalue::BinaryOp` shape our translator handles; with overflow checks on,
/// `a + b` lowers to `CheckedBinaryOp` plus a synthesized overflow `Assert`,
/// which is unrelated to the obligations the source actually states.
fn compiler_args(path: &Path) -> Result<Vec<String>, String> {
    let out_dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    Ok(vec![
        "rustc".to_string(),
        "--crate-type".to_string(),
        "lib".to_string(),
        "--edition".to_string(),
        "2021".to_string(),
        "-C".to_string(),
        "overflow-checks=off".to_string(),
        "--emit".to_string(),
        "metadata".to_string(),
        "--out-dir".to_string(),
        out_dir.display().to_string(),
        "--sysroot".to_string(),
        sysroot()?,
        path.display().to_string(),
    ])
}

/// Resolve the active toolchain's sysroot. The internal compilation
/// triggered by `rustc_public::run!` needs this passed explicitly — without
/// it, the child compilation can resolve a different `std`/`core` than the
/// one this very binary was linked against, producing duplicate-crate
/// ("only shows up once") errors.
fn sysroot() -> Result<String, String> {
    let output = std::process::Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .map_err(|e| format!("could not run `rustc --print sysroot`: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "`rustc --print sysroot` failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn collect_and_verify() -> ControlFlow<(), Vec<(String, Result<VerificationReport, String>)>> {
    let mut results = Vec::new();
    for item in rustc_public::all_local_items() {
        if item.kind() != ItemKind::Fn {
            continue;
        }
        let Some(sm_body) = item.body() else {
            continue;
        };
        let name = item.trimmed_name();
        let result = translate_body(&sm_body).and_then(|mir_body| {
            verify::verify_body(&mir_body).map_err(|e| e.to_string())
        });
        results.push((name, result));
    }
    ControlFlow::Continue(results)
}

/// Translate a real `stable_mir` function body into our MIR-lite
/// `MirBody`, or report why it falls outside the translatable subset
/// (see `mir.rs`'s module docs: no loops, whole-local places only, and only
/// the `Add`/`Mul`/`Le` binary operators).
fn translate_body(body: &rustc_public::mir::Body) -> Result<MirBody, String> {
    let locals: Vec<Local> = body
        .locals()
        .iter()
        .enumerate()
        .map(|(i, l)| Local::new(format!("_{i}"), map_ty(&l.ty)))
        .collect();
    let args: Vec<LocalId> = (0..body.arg_locals().len())
        .map(|i| LocalId((i + 1) as u32))
        .collect();

    // Only translate blocks the modeled control flow can actually reach.
    // Real MIR includes panic/unwind blocks (calls into
    // `core::panicking::panic` and friends) that are outside our
    // translatable subset but are also never visited by
    // `verify::verify_body`'s own walk — eagerly translating them would
    // reject perfectly good bodies over code nothing ever checks.
    let reachable = reachable_blocks(body);
    let blocks = body
        .blocks
        .iter()
        .enumerate()
        .map(|(idx, bb)| {
            if reachable.contains(&idx) {
                translate_block(bb)
            } else {
                Ok(BasicBlock {
                    statements: Vec::new(),
                    terminator: Terminator::Return,
                })
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(MirBody {
        args,
        locals,
        blocks,
    })
}

/// Blocks reachable from block 0 by following only the edges our translator
/// models: `Goto`, `Assert`'s success `target`, and a `SwitchInt`'s
/// `otherwise` edge (the assert-shape's success path — see
/// `translate_terminator`). Any other terminator (e.g. a `Call` into a panic
/// path) is treated as a dead end, matching `verify::verify_body`'s own walk.
fn reachable_blocks(body: &rustc_public::mir::Body) -> std::collections::HashSet<usize> {
    use rustc_public::mir::TerminatorKind;

    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![0usize];
    while let Some(idx) = stack.pop() {
        if !visited.insert(idx) {
            continue;
        }
        let Some(bb) = body.blocks.get(idx) else {
            continue;
        };
        match &bb.terminator.kind {
            TerminatorKind::Goto { target } => stack.push(*target),
            TerminatorKind::Assert { target, .. } => stack.push(*target),
            TerminatorKind::SwitchInt { targets, .. } => stack.push(targets.otherwise()),
            _ => {}
        }
    }
    visited
}

fn translate_block(block: &rustc_public::mir::BasicBlock) -> Result<BasicBlock, String> {
    use rustc_public::mir::StatementKind;

    let mut statements = Vec::new();
    for stmt in &block.statements {
        match &stmt.kind {
            StatementKind::Assign(place, rvalue) => {
                statements.push(Statement::Assign {
                    place: translate_place(place)?,
                    rvalue: translate_rvalue(rvalue)?,
                });
            }
            StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop => {}
            other => return Err(format!("unsupported statement: {other:?}")),
        }
    }

    let terminator = translate_terminator(&block.terminator.kind)?;
    Ok(BasicBlock {
        statements,
        terminator,
    })
}

fn translate_terminator(kind: &rustc_public::mir::TerminatorKind) -> Result<Terminator, String> {
    use rustc_public::mir::TerminatorKind;

    match kind {
        TerminatorKind::Return => Ok(Terminator::Return),
        TerminatorKind::Goto { target } => Ok(Terminator::Goto(BasicBlockId(*target as u32))),
        TerminatorKind::Assert {
            cond,
            expected,
            target,
            ..
        } => Ok(Terminator::Assert {
            condition: translate_operand(cond)?,
            expected: *expected,
            target: BasicBlockId(*target as u32),
        }),
        // A user-written `assert!(cond)` (as opposed to a compiler-inserted
        // bounds/overflow check) lowers to `if !cond { panic!() }`, i.e. a
        // `bool` `SwitchInt` with exactly one explicit branch — not a native
        // `Assert` terminator. `branches() == [(0, panic_bb)]` with
        // `otherwise = continue_bb` means "continue iff `discr != 0`", the
        // same obligation an `Assert{cond: discr, expected: true, ...}`
        // states; we translate it identically and simply never visit the
        // (unmodeled) panic block.
        TerminatorKind::SwitchInt { discr, targets } => {
            let branches: Vec<_> = targets.branches().collect();
            match branches.as_slice() {
                [(0, _panic_target)] => Ok(Terminator::Assert {
                    condition: translate_operand(discr)?,
                    expected: true,
                    target: BasicBlockId(targets.otherwise() as u32),
                }),
                [(1, _panic_target)] => Ok(Terminator::Assert {
                    condition: translate_operand(discr)?,
                    expected: false,
                    target: BasicBlockId(targets.otherwise() as u32),
                }),
                _ => Err(format!(
                    "unsupported switch (only a single-branch bool assert-shape is modeled): {kind:?}"
                )),
            }
        }
        other => Err(format!("unsupported terminator: {other:?}")),
    }
}

fn translate_place(place: &rustc_public::mir::Place) -> Result<Place, String> {
    if !place.projection.is_empty() {
        return Err(format!(
            "place projections are not supported: {place:?}"
        ));
    }
    Ok(Place(LocalId(place.local as u32)))
}

fn translate_operand(op: &rustc_public::mir::Operand) -> Result<Operand, String> {
    use rustc_public::mir::Operand as SmOperand;

    match op {
        SmOperand::Copy(place) | SmOperand::Move(place) => {
            Ok(Operand::Local(translate_place(place)?.0))
        }
        SmOperand::Constant(c) => match c.const_.eval_target_usize() {
            Ok(n) => Ok(Operand::NatConst(n)),
            Err(_) => Err(format!(
                "unsupported constant (only unsigned integers are modeled): {c:?}"
            )),
        },
        other => Err(format!("unsupported operand: {other:?}")),
    }
}

fn translate_rvalue(rv: &rustc_public::mir::Rvalue) -> Result<Rvalue, String> {
    use rustc_public::mir::BinOp as SmBinOp;
    use rustc_public::mir::Rvalue as SmRvalue;

    match rv {
        SmRvalue::Use(op, _) => Ok(Rvalue::Use(translate_operand(op)?)),
        SmRvalue::BinaryOp(binop, a, b) => {
            let op = match binop {
                SmBinOp::Add | SmBinOp::AddUnchecked => BinOp::Add,
                SmBinOp::Mul | SmBinOp::MulUnchecked => BinOp::Mul,
                SmBinOp::Le => BinOp::Le,
                other => return Err(format!("unsupported binary operator: {other:?}")),
            };
            Ok(Rvalue::BinaryOp(op, translate_operand(a)?, translate_operand(b)?))
        }
        other => Err(format!("unsupported rvalue: {other:?}")),
    }
}

fn collect_signatures() -> ControlFlow<(), Vec<FnSignature>> {
    let mut sigs = Vec::new();
    for item in rustc_public::all_local_items() {
        if item.kind() != ItemKind::Fn {
            continue;
        }
        let Some(body) = item.body() else {
            continue;
        };
        let args = body.arg_locals().iter().map(|l| map_ty(&l.ty)).collect();
        let ret = map_ty(&body.ret_local().ty);
        sigs.push(FnSignature {
            name: item.trimmed_name(),
            args,
            ret,
        });
    }
    ControlFlow::Continue(sigs)
}

/// Map a `rustc_public` `Ty` into our `RustType`. Only a subset of Rust's
/// type system is handled for 1.0 — anything else degrades to
/// `RustType::Generic` with a debug label rather than failing ingestion.
fn map_ty(ty: &Ty) -> RustType {
    match ty.kind() {
        TyKind::RigidTy(RigidTy::Bool) => RustType::Primitive("bool".to_string()),
        TyKind::RigidTy(RigidTy::Char) => RustType::Primitive("char".to_string()),
        TyKind::RigidTy(RigidTy::Int(i)) => RustType::Primitive(int_name(i)),
        TyKind::RigidTy(RigidTy::Uint(u)) => RustType::Primitive(uint_name(u)),
        TyKind::RigidTy(RigidTy::Tuple(tys)) if tys.is_empty() => {
            RustType::Primitive("()".to_string())
        }
        TyKind::RigidTy(RigidTy::Tuple(tys)) => {
            RustType::Tuple(tys.iter().map(map_ty).collect())
        }
        TyKind::RigidTy(RigidTy::Ref(_, inner, mutability)) => RustType::Reference(
            Box::new(map_ty(&inner)),
            matches!(mutability, Mutability::Mut),
        ),
        TyKind::RigidTy(RigidTy::Slice(inner)) => RustType::Slice(Box::new(map_ty(&inner))),
        TyKind::RigidTy(RigidTy::Never) => RustType::Never,
        other => RustType::Generic(format!("{other:?}")),
    }
}

fn int_name(i: IntTy) -> String {
    format!("{i:?}").to_lowercase()
}

fn uint_name(u: UintTy) -> String {
    format!("{u:?}").to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_sample(dir: &Path, src: &str) -> std::path::PathBuf {
        let path = dir.join("sample.rs");
        std::fs::write(&path, src).unwrap();
        path
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tpt-mir-ingest-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn ingests_primitive_signatures() {
        let dir = temp_dir("primitives");
        let path = write_sample(
            &dir,
            r#"
                pub fn add(a: u32, b: u32) -> u32 { a + b }
                pub fn negate(x: bool) -> bool { !x }
            "#,
        );

        let sigs = ingest_file(&path).unwrap();
        let add = sigs.iter().find(|s| s.name == "add").unwrap();
        assert_eq!(
            add.args,
            vec![
                RustType::Primitive("u32".to_string()),
                RustType::Primitive("u32".to_string())
            ]
        );
        assert_eq!(add.ret, RustType::Primitive("u32".to_string()));

        let negate = sigs.iter().find(|s| s.name == "negate").unwrap();
        assert_eq!(negate.args, vec![RustType::Primitive("bool".to_string())]);
        assert_eq!(negate.ret, RustType::Primitive("bool".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ingests_reference_and_tuple_types() {
        let dir = temp_dir("refs-tuples");
        let path = write_sample(
            &dir,
            r#"
                pub fn first(pair: &(u32, bool)) -> u32 { pair.0 }
            "#,
        );

        let sigs = ingest_file(&path).unwrap();
        let first = sigs.iter().find(|s| s.name == "first").unwrap();
        assert_eq!(
            first.args,
            vec![RustType::Reference(
                Box::new(RustType::Tuple(vec![
                    RustType::Primitive("u32".to_string()),
                    RustType::Primitive("bool".to_string()),
                ])),
                false,
            )]
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_source_that_does_not_compile() {
        let dir = temp_dir("bad-source");
        let path = write_sample(&dir, "pub fn broken( -> {");

        assert!(ingest_file(&path).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Milestone 3: verify a simple real Rust function end-to-end — real
    /// source, compiled by the pinned nightly, its actual MIR ingested via
    /// `stable_mir`/`rustc_public`, translated into `tpt_rustir_mir::mir`,
    /// and its `assert!`'s obligation discharged by the same kernel/tactic
    /// pipeline the `#[tpt::spec]`/`#[tpt::verify]` proc-macros use.
    #[test]
    fn verifies_a_simple_rust_function_end_to_end() {
        let dir = temp_dir("milestone3");
        let path = write_sample(
            &dir,
            r#"
                pub fn sum_order_bound(a: u32, b: u32) -> u32 {
                    assert!(a + b <= b + a);
                    a + b
                }
            "#,
        );

        let results = ingest_and_verify_file(&path).unwrap();
        let (_, result) = results
            .iter()
            .find(|(name, _)| name == "sum_order_bound")
            .expect("sum_order_bound was ingested");
        let report = result.as_ref().expect("body is in the translatable subset");
        assert!(
            report.all_discharged(),
            "expected every obligation discharged, got {report:?}"
        );
        assert_eq!(report.outcomes.len(), 1, "expected exactly one assert obligation");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A genuinely false obligation, ingested from real source, must still
    /// be rejected — the pipeline doesn't just rubber-stamp everything it
    /// can translate.
    #[test]
    fn rejects_an_unprovable_obligation_end_to_end() {
        let dir = temp_dir("milestone3-false");
        let path = write_sample(
            &dir,
            r#"
                pub fn bogus_bound(a: u32, b: u32) -> u32 {
                    assert!(a <= b);
                    a + b
                }
            "#,
        );

        let results = ingest_and_verify_file(&path).unwrap();
        let (_, result) = results
            .iter()
            .find(|(name, _)| name == "bogus_bound")
            .expect("bogus_bound was ingested");
        assert!(result.is_err(), "expected the false obligation to be rejected");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
