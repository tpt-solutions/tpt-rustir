//! Extraction: translating verified tpt terms back into Rust source.
//!
//! Extraction erases the parts of the dependent type theory that Rust's type
//! system cannot express:
//! - dependent pairs (Sigma) become plain tuples `(A, B)` or generated
//!   structs — when the second component's type genuinely depends on the
//!   first's *value*, the dependency is erased (Rust has no refinement
//!   types); a concrete first-component value can be supplied to resolve the
//!   dependency before erasure;
//! - dependent functions (Pi) become plain `fn` signatures — a codomain that
//!   mentions the domain binder cannot be extracted and is reported as an
//!   error rather than silently erased.

use tpt_rustir_core::env::GlobalEnv;
use tpt_rustir_core::term::{self, Term, TermKind};
use tpt_rustir_core::whnf;

use crate::mir::{MirBody, Operand, Rvalue, Statement, Terminator};

#[derive(Debug, Clone)]
pub enum ExtractError {
    /// The core type is not expressible as a Rust type.
    UnsupportedType { term: String, reason: String },
    /// A dependent field/function codomain mentions its binder and no
    /// witness was supplied to resolve the dependency.
    DependencyNotResolvable { term: String },
    /// Not a Sigma/pair-shaped term.
    NotAPairType { term: String },
    /// Not a Pi-shaped term.
    NotAPiType { term: String },
    /// A value term is not a closed Nat/Bool/pair value.
    UnsupportedValue { term: String },
    /// The body has no reachable `Return` or no return local.
    NoReturnPlace,
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractError::UnsupportedType { term, reason } => {
                write!(f, "cannot extract type `{term}`: {reason}")
            }
            ExtractError::DependencyNotResolvable { term } => write!(
                f,
                "dependent type `{term}` mentions its binder; supply a witness or use \
                 struct extraction with the dependency erased"
            ),
            ExtractError::NotAPairType { term } => {
                write!(f, "expected a Sigma type, found `{term}`")
            }
            ExtractError::NotAPiType { term } => {
                write!(f, "expected a Pi type, found `{term}`")
            }
            ExtractError::UnsupportedValue { term } => {
                write!(f, "cannot extract value `{term}`")
            }
            ExtractError::NoReturnPlace => write!(f, "body has no local 0 return place"),
        }
    }
}
impl std::error::Error for ExtractError {}

/// Whether the free binder at de Bruijn depth `depth` occurs in `t`.
/// Occurrences of `Var(n)` with `n >= depth` are free at this depth.
fn mentions_binder(t: &Term, depth: usize) -> bool {
    match &**t {
        TermKind::Var(n) => *n >= depth,
        TermKind::Const(_)
        | TermKind::Universe(_)
        | TermKind::Nat
        | TermKind::Zero
        | TermKind::Bool
        | TermKind::True
        | TermKind::False => false,
        TermKind::Pi(dom, cod) | TermKind::Sigma(dom, cod) => {
            mentions_binder(dom, depth) || mentions_binder(cod, depth + 1)
        }
        TermKind::Lambda(dom, body) => {
            mentions_binder(dom, depth) || mentions_binder(body, depth + 1)
        }
        TermKind::App(f, a) => mentions_binder(f, depth) || mentions_binder(a, depth),
        TermKind::Pair(a, b, ty) => {
            mentions_binder(a, depth) || mentions_binder(b, depth) || mentions_binder(ty, depth)
        }
        TermKind::Fst(p) | TermKind::Snd(p) | TermKind::Succ(p) => mentions_binder(p, depth),
        TermKind::Inductive(_, params) => params.iter().any(|p| mentions_binder(p, depth)),
        TermKind::Con(_, _, args) => args.iter().any(|a| mentions_binder(a, depth)),
        TermKind::IndRec(_, motive, cases, scrutinee) => {
            mentions_binder(motive, depth)
                || cases.iter().any(|(_, b)| mentions_binder(b, depth + 1))
                || mentions_binder(scrutinee, depth)
        }
        TermKind::NatRec(m, b, s, t0) | TermKind::BoolRec(m, b, s, t0) => {
            mentions_binder(m, depth)
                || mentions_binder(b, depth)
                || mentions_binder(s, depth)
                || mentions_binder(t0, depth)
        }
        TermKind::Let(ty, val, body) => {
            mentions_binder(ty, depth)
                || mentions_binder(val, depth)
                || mentions_binder(body, depth + 1)
        }
    }
}

/// The value of a closed Nat term (a `succ`-chain over `zero`).
pub fn nat_value(t: &Term) -> Option<u64> {
    match &**t {
        TermKind::Zero => Some(0),
        TermKind::Succ(n) => nat_value(n).map(|v| v + 1),
        _ => None,
    }
}

/// Sanitize a name into a legal Rust identifier.
fn sanitize_ident(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// The Rust source for a *core* type: kernel Nat maps to `u32`, kernel Bool
/// to `bool`, and stdlib primitive aliases to their Rust names.
pub fn rust_type_source(env: &GlobalEnv, ty: &Term) -> Result<String, ExtractError> {
    let ty = whnf(env, ty);
    match &*ty {
        TermKind::Nat => Ok("u32".to_string()),
        TermKind::Bool => Ok("bool".to_string()),
        TermKind::Const(name) => {
            if let Some(src) = crate::stdlib::alias_source_name(name) {
                Ok(src.to_string())
            } else {
                Ok(sanitize_ident(name))
            }
        }
        TermKind::Sigma(..) => pair_type_to_tuple_source(env, &ty).map(|t| t.source),
        _ => Err(ExtractError::UnsupportedType {
            term: tpt_rustir_syntax::pretty::print(&ty),
            reason: "only Nat/Bool/aliases/tuples extract for 1.0".to_string(),
        }),
    }
}

/// The extraction of a Sigma type as a Rust tuple type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleExtraction {
    /// e.g. `"(u32, bool)"`.
    pub source: String,
    /// `true` when a genuine binder dependency was resolved by substituting
    /// the supplied witness; `false` when the type had no (or a vacuous)
    /// dependency.
    pub dependency_resolved: bool,
}

/// Whether `Var(target)` occurs in `t`, where `target` counts the binders
/// entered so far (binders inside `t` shift the index). Unlike
/// `mentions_binder`, this matches *exactly* one binder, ignoring outer
/// free variables.
fn mentions_exact(t: &Term, target: usize) -> bool {
    match &**t {
        TermKind::Var(n) => *n == target,
        TermKind::Const(_)
        | TermKind::Universe(_)
        | TermKind::Nat
        | TermKind::Zero
        | TermKind::Bool
        | TermKind::True
        | TermKind::False => false,
        TermKind::Pi(dom, cod) | TermKind::Sigma(dom, cod) => {
            mentions_exact(dom, target) || mentions_exact(cod, target + 1)
        }
        TermKind::Lambda(dom, body) => {
            mentions_exact(dom, target) || mentions_exact(body, target + 1)
        }
        TermKind::App(f, a) => mentions_exact(f, target) || mentions_exact(a, target),
        TermKind::Pair(a, b, ty) => {
            mentions_exact(a, target) || mentions_exact(b, target) || mentions_exact(ty, target)
        }
        TermKind::Fst(p) | TermKind::Snd(p) | TermKind::Succ(p) => mentions_exact(p, target),
        TermKind::Inductive(_, params) => params.iter().any(|p| mentions_exact(p, target)),
        TermKind::Con(_, _, args) => args.iter().any(|a| mentions_exact(a, target)),
        TermKind::IndRec(_, motive, cases, scrutinee) => {
            mentions_exact(motive, target)
                || cases.iter().any(|(_, b)| mentions_exact(b, target + 1))
                || mentions_exact(scrutinee, target)
        }
        TermKind::NatRec(m, b, s, t0) | TermKind::BoolRec(m, b, s, t0) => {
            mentions_exact(m, target)
                || mentions_exact(b, target)
                || mentions_exact(s, target)
                || mentions_exact(t0, target)
        }
        TermKind::Let(ty, val, body) => {
            mentions_exact(ty, target)
                || mentions_exact(val, target)
                || mentions_exact(body, target + 1)
        }
    }
}

/// Translate a (possibly dependent) pair type into a Rust tuple type.
///
/// The dependency is erased: if any component's type mentions an earlier
/// component's binder, a `witness` value must be supplied to resolve the
/// dependency before erasure (e.g. `zero`), otherwise
/// `DependencyNotResolvable` is returned.
pub fn pair_type_to_tuple_source(
    env: &GlobalEnv,
    ty: &Term,
) -> Result<TupleExtraction, ExtractError> {
    pair_type_to_tuple_source_with_witness(env, ty, None)
}

/// As `pair_type_to_tuple_source`, with an optional witness for the first
/// component's value (used to resolve genuine dependencies).
pub fn pair_type_to_tuple_source_with_witness(
    env: &GlobalEnv,
    ty: &Term,
    witness: Option<&Term>,
) -> Result<TupleExtraction, ExtractError> {
    let mut components: Vec<String> = Vec::new();
    let mut current = whnf(env, ty).clone();
    let mut dependency_resolved = false;
    let mut saw_sigma = false;
    // Witness for each binder already walked, in walk order (binder j's
    // witness = the j-th component's value). `None` = no dependency, no
    // witness.
    let mut witnesses: Vec<Option<Term>> = Vec::new();

    loop {
        let cur = whnf(env, &current);
        let (a_ty, next) = match &*cur {
            TermKind::Sigma(a_ty, b_ty) => {
                saw_sigma = true;
                (Some(a_ty.clone()), Some(b_ty.clone()))
            }
            _ => (Some(cur.clone()), None),
        };
        let a_ty = a_ty.unwrap();
        // `a_ty` is the next component's type with the previously
        // walked binders in scope. De Bruijn index 0 is the
        // *nearest* binder — the most recently walked component's
        // binder — so walked component `j`'s binder sits at index
        // `binders - 1 - j`. Resolve every mentioned binder with its
        // witness (processing indices innermost-out; `subst` at one
        // depth leaves shallower indices untouched).
        let binders = witnesses.len();
        let mut resolved = a_ty;
        let mut used_witness = false;
        for i in 0..binders {
            if mentions_exact(&resolved, i) {
                let walk_idx = binders - 1 - i;
                match &witnesses[walk_idx] {
                    Some(w) => {
                        resolved = term::subst(&resolved, i, w);
                        used_witness = true;
                    }
                    None => {
                        return Err(ExtractError::DependencyNotResolvable {
                            term: tpt_rustir_syntax::pretty::print(&cur),
                        })
                    }
                }
            }
        }
        let resolved = whnf(env, &resolved);
        components.push(rust_type_source(env, &resolved)?);
        dependency_resolved |= used_witness;
        match next {
            Some(b_ty) => {
                // Record this binder's witness for later components. The
                // caller's `witness` applies to the *first* walked component
                // only; later components have no witness.
                witnesses.push(if witnesses.is_empty() { witness.cloned() } else { None });
                current = b_ty;
            }
            None => break,
        }
    }

    if !saw_sigma || components.is_empty() {
        return Err(ExtractError::NotAPairType {
            term: tpt_rustir_syntax::pretty::print(&whnf(env, ty)),
        });
    }
    Ok(TupleExtraction {
        source: format!("({})", components.join(", ")),
        dependency_resolved,
    })
}

/// Translate a (possibly dependent) pair type into a generated Rust struct.
///
/// The dependency between fields is erased (Rust has no refinement types);
/// a witness, when supplied, resolves it before erasure and the struct's doc
/// comment records that the verified invariant is enforced by the proof, not
/// by the Rust type.
pub fn pair_type_to_struct_source(
    env: &GlobalEnv,
    name: &str,
    ty: &Term,
) -> Result<String, ExtractError> {
    let tuple = pair_type_to_tuple_source(env, ty)?;
    let inner = tuple
        .source
        .trim_start_matches('(')
        .trim_end_matches(')')
        .to_string();
    let fields: Vec<String> = inner
        .split(", ")
        .enumerate()
        .map(|(i, f)| format!("    pub field{i}: {f},"))
        .collect();
    Ok(format!(
        "/// Extracted from the tpt pair type `{}`.\n/// Note: any dependency between the fields is erased in Rust's type\n/// system; the verified invariant is enforced by the accompanying proof.\n#[derive(Debug, Clone, PartialEq, Eq)]\npub struct {} {{\n{}\n}}",
        tpt_rustir_syntax::pretty::print(&whnf(env, ty)),
        sanitize_ident(name),
        fields.join("\n")
    ))
}

/// Translate a pair *value* term into a Rust tuple expression, e.g.
/// `(zero, succ zero)` → `"(0, 1)"`.
pub fn pair_value_to_rust(t: &Term) -> Result<String, ExtractError> {
    match &**t {
        TermKind::Zero => Ok("0".to_string()),
        TermKind::Succ(_) => nat_value(t)
            .map(|n| n.to_string())
            .ok_or_else(|| ExtractError::UnsupportedValue {
                term: tpt_rustir_syntax::pretty::print(t),
            }),
        TermKind::True => Ok("true".to_string()),
        TermKind::False => Ok("false".to_string()),
        TermKind::Pair(a, b, _) => Ok(format!(
            "({}, {})",
            pair_value_to_rust(a)?,
            pair_value_to_rust(b)?
        )),
        _ => Err(ExtractError::UnsupportedValue {
            term: tpt_rustir_syntax::pretty::print(t),
        }),
    }
}

/// Translate a (non-dependent) Pi type into a Rust `fn` type source, e.g.
/// `(x : Nat) -> Nat` → `"fn(u32) -> u32"`. A codomain that mentions the
/// domain binder cannot be expressed in Rust and is reported as
/// `DependencyNotResolvable`.
pub fn pi_type_to_fn_source(env: &GlobalEnv, ty: &Term) -> Result<String, ExtractError> {
    let mut args: Vec<String> = Vec::new();
    let mut current = whnf(env, ty).clone();
    loop {
        let cur = whnf(env, &current);
        match &*cur {
            TermKind::Pi(dom, cod) => {
                // For 1.0 we extract closed signatures only: a domain that
                // mentions any binder in scope, or a codomain that mentions
                // this Pi's binder, is a genuine dependency Rust cannot
                // express — report it instead of silently erasing.
                if mentions_binder(dom, 0) || mentions_exact(cod, 0) {
                    return Err(ExtractError::DependencyNotResolvable {
                        term: tpt_rustir_syntax::pretty::print(&cur),
                    });
                }
                args.push(rust_type_source(env, dom)?);
                current = cod.clone();
            }
            _ => break,
        }
    }
    if args.is_empty() {
        return Err(ExtractError::NotAPiType {
            term: tpt_rustir_syntax::pretty::print(&whnf(env, ty)),
        });
    }
    let ret = rust_type_source(env, &current)?;
    Ok(format!("fn({}) -> {ret}", args.join(", ")))
}

/// The Rust source for a `RustType` (used for MIR locals).
fn rust_local_type_source(ty: &crate::RustType) -> String {
    use crate::RustType::*;
    match ty {
        Primitive(name) => name.clone(),
        Reference(inner, is_mut) => {
            if *is_mut {
                format!("&mut {}", rust_local_type_source(inner))
            } else {
                format!("&{}", rust_local_type_source(inner))
            }
        }
        Tuple(ts) => format!(
            "({})",
            ts.iter()
                .map(rust_local_type_source)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Array(inner, n) => format!("[{}; {n}]", rust_local_type_source(inner)),
        Slice(inner) => format!("&[{}]", rust_local_type_source(inner)),
        Function(args, ret) => format!(
            "fn({}) -> {}",
            args.iter()
                .map(rust_local_type_source)
                .collect::<Vec<_>>()
                .join(", "),
            rust_local_type_source(ret)
        ),
        Adt(name, params) => {
            if params.is_empty() {
                name.clone()
            } else {
                format!(
                    "{}<{}>",
                    name,
                    params
                        .iter()
                        .map(rust_local_type_source)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
        Generic(name) => name.clone(),
        Never => "!".to_string(),
    }
}

/// The Rust source for an operand.
fn operand_source(body: &MirBody, op: &Operand) -> Result<String, ExtractError> {
    Ok(match op {
        Operand::NatConst(n) => n.to_string(),
        Operand::BoolConst(b) => b.to_string(),
        Operand::Local(id) => body
            .local(*id)
            .map(|l| sanitize_ident(&l.name))
            .unwrap_or_else(|| format!("_{}", id.0)),
    })
}

/// The Rust source for an rvalue.
fn rvalue_source(body: &MirBody, rv: &Rvalue) -> Result<String, ExtractError> {
    match rv {
        Rvalue::Use(op) => operand_source(body, op),
        Rvalue::BinaryOp(op, a, b) => Ok(format!(
            "({} {} {})",
            operand_source(body, a)?,
            op.rust_symbol(),
            operand_source(body, b)?
        )),
    }
}

fn statement_source(
    body: &MirBody,
    statement: &Statement,
    indent: &str,
    out: &mut String,
) -> Result<(), ExtractError> {
    match statement {
        Statement::Assign { place, rvalue } => {
            let name = body
                .local(place.0)
                .map(|l| sanitize_ident(&l.name))
                .unwrap_or_else(|| format!("_{}", place.0 .0));
            out.push_str(&format!(
                "{indent}let {} = {};\n",
                name,
                rvalue_source(body, rvalue)?
            ));
        }
        Statement::Unsafe {
            description,
            statements,
            ..
        } => {
            out.push_str(&format!(
                "{indent}// SAFETY: justified by the tpt proof obligation `{description}`\n"
            ));
            out.push_str(&format!("{indent}unsafe {{\n"));
            for s in statements {
                statement_source(body, s, &format!("{indent}    "), out)?;
            }
            out.push_str(&format!("{indent}}}\n"));
        }
    }
    Ok(())
}

/// Extract a verified MIR body back into readable Rust source.
///
/// This is source-level extraction for 1.0: emitting readable, checkable
/// Rust text rather than constructing compiler `syn`/MIR nodes directly
/// (real MIR construction needs the compiler APIs; see `todo.md`).
pub fn body_to_rust_source(name: &str, body: &MirBody) -> Result<String, ExtractError> {
    let ret_local = body.return_local().ok_or(ExtractError::NoReturnPlace)?;

    let args: Vec<String> = body
        .args
        .iter()
        .filter_map(|id| {
            body.local(*id).map(|l| {
                format!(
                    "{}: {}",
                    sanitize_ident(&l.name),
                    rust_local_type_source(&l.ty)
                )
            })
        })
        .collect();

    let mut out = String::new();
    out.push_str(&format!(
        "pub fn {}({}) -> {} {{\n",
        sanitize_ident(name),
        args.join(", "),
        rust_local_type_source(&ret_local.ty)
    ));

    // Walk the reachable block chain, emitting statements.
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![0u32];
    while let Some(bid) = stack.pop() {
        if !visited.insert(bid) {
            continue;
        }
        let block = body
            .blocks
            .get(bid as usize)
            .ok_or(ExtractError::NoReturnPlace)?;
        for s in &block.statements {
            statement_source(body, s, "    ", &mut out)?;
        }
        match &block.terminator {
            Terminator::Return => {
                out.push_str(&format!("    {}\n", sanitize_ident(&ret_local.name)));
            }
            Terminator::Goto(target) => stack.push(target.0),
            Terminator::Assert {
                condition,
                expected,
                target,
            } => {
                out.push_str(&format!(
                    "    debug_assert_eq!({}, {});\n",
                    operand_source(body, condition)?,
                    expected
                ));
                stack.push(target.0);
            }
        }
    }

    out.push_str("}\n");
    Ok(out)
}