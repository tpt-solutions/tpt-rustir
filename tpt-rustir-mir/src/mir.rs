//! A MIR-lite representation of Rust function bodies, covering the
//! arithmetic/control subset `tpt-rustir` verifies today.
//!
//! This is the *sample* body model used by Phase 3 until `stable_mir`
//! ingestion lands (which requires building `stable_mir` from the rust
//! compiler source — see `todo.md`). The shape deliberately mirrors real MIR:
//! locals, basic blocks, statement/terminator split — so the Phase 3
//! `stable_mir` adapter can translate 1:1 into this model.
//!
//! Scope notes:
//! - no loops are modeled yet (`Goto` back edges terminate verification via
//!   a visited set);
//! - `Assert` models panicking assertions (`index` bounds checks, arithmetic
//!   overflow checks) and generates a proof obligation;
//! - `Unsafe` models `unsafe { … }` blocks, which must be justified by a
//!   logical proof of their safety obligation (see `verify.rs`).

use crate::RustType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BasicBlockId(pub u32);

/// A MIR local: a named, typed slot.
#[derive(Debug, Clone)]
pub struct Local {
    pub name: String,
    pub ty: RustType,
}

impl Local {
    pub fn new(name: impl Into<String>, ty: RustType) -> Self {
        Local {
            name: name.into(),
            ty,
        }
    }
}

/// An operand: a constant or a read of a local.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    NatConst(u64),
    BoolConst(bool),
    Local(LocalId),
}

/// The binary operators the bridge can reason about, each backed by a stdlib
/// constant in `tpt_rustir_mir::stdlib`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Mul,
    Le,
}

impl BinOp {
    /// The stdlib constant computing this operator.
    pub fn stdlib_const(self) -> &'static str {
        match self {
            BinOp::Add => "add",
            BinOp::Mul => "mul",
            BinOp::Le => "le",
        }
    }

    /// The Rust surface symbol for extraction.
    pub fn rust_symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Mul => "*",
            BinOp::Le => "<=",
        }
    }
}

/// The right-hand side of an assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rvalue {
    Use(Operand),
    BinaryOp(BinOp, Operand, Operand),
}

/// An assignment destination (whole-local places only for 1.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place(pub LocalId);

/// A statement: effectful computation inside a block.
#[derive(Debug, Clone)]
pub enum Statement {
    /// `place = rvalue;`
    Assign { place: Place, rvalue: Rvalue },
    /// `unsafe { statements }` — must be justified by a logical proof of
    /// `obligation` (a kernel proposition, i.e. a `Bool`-valued term).
    ///
    /// `assumption` optionally records an *opaque hypothesis* (axiom) the
    /// proof may rely on; it is trusted but recorded in the verification
    /// report, mirroring how real verifiers treat explicit assumptions.
    Unsafe {
        description: String,
        obligation: tpt_rustir_core::Term,
        assumption: Option<tpt_rustir_core::Term>,
        proof: Option<tpt_rustir_core::Term>,
        statements: Vec<Statement>,
    },
}

/// A block terminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminator {
    /// Exit the function (the return value is local 0, as in real MIR).
    Return,
    /// Unconditional jump.
    Goto(BasicBlockId),
    /// `assert(condition == expected)` — e.g. a bounds check. Generates a
    /// proof obligation; control continues at `target` when it holds.
    Assert {
        condition: Operand,
        expected: bool,
        target: BasicBlockId,
    },
}

/// A basic block: straight-line statements followed by one terminator.
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub statements: Vec<Statement>,
    pub terminator: Terminator,
}

/// A function body: locals, arguments (as local ids), and basic blocks.
///
/// By convention (mirroring real MIR) local 0 is the return place `_0`, and
/// the remaining locals are temporaries. Arguments are the locals listed in
/// `args`, in declaration order.
#[derive(Debug, Clone, Default)]
pub struct MirBody {
    pub args: Vec<LocalId>,
    pub locals: Vec<Local>,
    pub blocks: Vec<BasicBlock>,
}

impl MirBody {
    /// The local declared with `id`, if any.
    pub fn local(&self, id: LocalId) -> Option<&Local> {
        self.locals.get(id.0 as usize)
    }

    /// The return-place local (`_0`), if declared.
    pub fn return_local(&self) -> Option<&Local> {
        self.locals.first()
    }

    /// Type of a local, if declared.
    pub fn local_ty(&self, id: LocalId) -> Option<&RustType> {
        self.local(id).map(|l| &l.ty)
    }
}
