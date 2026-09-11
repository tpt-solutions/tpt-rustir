//! SMT fallback for obligations equality saturation can't discharge, e.g.
//! genuine arithmetic reasoning. Gated behind the `smt` feature since it
//! links against a system Z3 install (not present in CI).
//!
//! Obligations arrive as `tactic::Goal`s (or parsed `RecExpr<Expr>`s) in the
//! same `Expr` language the e-graph uses. They are translated to Z3 `Int` /
//! `Bool` trees by inferring a sort for every node, then the goal `lhs = rhs`
//! is proved by asking Z3 whether `lhs ≠ rhs` is unsatisfiable.
//!
//! Well-sortedness is assumed and checked: the `Expr` language mixes an Int
//! sort (`zero`, `succ`, `+`, `*` and free symbols in arithmetic position)
//! with a Bool sort (`true`, `false`, `and`, `or`, `not`, `implies`, `le`).
//! A symbol used in both sorts is rejected as ill-sorted.

use std::collections::HashMap;
use std::str::FromStr;

use egg::{Id, RecExpr, Symbol};
use z3::{
    ast::{Bool, Int},
    Config, Context, SatResult, Solver,
};

use crate::lang::Expr;
use crate::simplify::ParseError;
use crate::tactic::Goal;

/// The two sorts appearing in `Expr`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Int,
    Bool,
}

impl std::fmt::Display for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sort::Int => write!(f, "Int"),
            Sort::Bool => write!(f, "Bool"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SmtError {
    Parse(ParseError),
    /// The two sides of the goal have different sorts.
    KindMismatch { lhs: Sort, rhs: Sort },
    /// A symbol is used in both an Int and a Bool position.
    IllSorted { symbol: String, second_use: Sort },
    /// Z3 returned `unknown` (timeout, undecidable fragment, …).
    Unknown,
    /// Z3 found a counterexample: the goal is false.
    Counterexample,
}

impl std::fmt::Display for SmtError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmtError::Parse(e) => write!(f, "{e}"),
            SmtError::KindMismatch { lhs, rhs } => {
                write!(f, "goal sides have different sorts: {lhs} vs {rhs}")
            }
            SmtError::IllSorted { symbol, second_use } => write!(
                f,
                "symbol `{symbol}` used in both Int and {second_use} positions"
            ),
            SmtError::Unknown => write!(f, "z3 could not decide the obligation"),
            SmtError::Counterexample => write!(f, "z3 found a counterexample"),
        }
    }
}
impl std::error::Error for SmtError {}
impl From<ParseError> for SmtError {
    fn from(e: ParseError) -> Self {
        SmtError::Parse(e)
    }
}

/// Infer the sort of the subtree rooted at `id`, recording the sort of every
/// free symbol on first use.
fn type_tree(
    expr: &RecExpr<Expr>,
    id: Id,
    sorts: &mut HashMap<Symbol, Sort>,
) -> Result<Sort, SmtError> {
    let node = &expr.as_ref()[usize::from(id)];
    let sort = match node {
        Expr::Zero | Expr::Succ(_) | Expr::Add(_) | Expr::Mul(_) => Sort::Int,
        Expr::True | Expr::False | Expr::And(_) | Expr::Or(_) | Expr::Not(_) | Expr::Implies(_)
        | Expr::Le(_) => Sort::Bool,
        Expr::Symbol(s) => {
            if let Some(existing) = sorts.get(s) {
                return Ok(*existing);
            }
            // A bare symbol with no positional constraints defaults to Int;
            // a later Bool use then triggers the `IllSorted` error below.
            sorts.insert(*s, Sort::Int);
            return Ok(Sort::Int);
        }
    };
    // Check children's sorts.
    let children: Vec<Id> = match node {
        Expr::Succ([a]) | Expr::Not([a]) => vec![*a],
        Expr::Add([a, b]) | Expr::Mul([a, b]) | Expr::And([a, b]) | Expr::Or([a, b])
        | Expr::Implies([a, b]) | Expr::Le([a, b]) => vec![*a, *b],
        _ => vec![],
    };
    for child in children {
        let child_sort = type_tree(expr, child, sorts)?;
        if child_sort != sort {
            let symbol = match &expr.as_ref()[usize::from(child)] {
                Expr::Symbol(s) => s.to_string(),
                _ => "<non-symbol>".to_string(),
            };
            return Err(SmtError::IllSorted {
                symbol,
                second_use: child_sort,
            });
        }
    }
    Ok(sort)
}

/// A Z3 AST value in either sort.
enum ZAst<'c> {
    I(Int<'c>),
    B(Bool<'c>),
}

impl<'c> ZAst<'c> {
    fn as_int(self) -> Option<Int<'c>> {
        match self {
            ZAst::I(i) => Some(i),
            _ => None,
        }
    }
    fn as_bool(self) -> Option<Bool<'c>> {
        match self {
            ZAst::B(b) => Some(b),
            _ => None,
        }
    }
}

/// Translate the subtree at `id` into a Z3 AST of the sort previously
/// inferred for it by `type_tree`.
fn build<'c>(
    ctx: &'c Context,
    expr: &RecExpr<Expr>,
    id: Id,
    sorts: &HashMap<Symbol, Sort>,
    cache: &mut HashMap<Id, ZAst<'c>>,
) -> Result<ZAst<'c>, SmtError> {
    if let Some(hit) = cache.get(&id) {
        return Ok(match hit {
            ZAst::I(i) => ZAst::I(i.clone()),
            ZAst::B(b) => ZAst::B(b.clone()),
        });
    }
    let node = &expr.as_ref()[usize::from(id)];
    let value: ZAst<'c> = match node {
        Expr::Zero => ZAst::I(Int::from_i64(ctx, 0)),
        Expr::True => ZAst::B(Bool::from_bool(ctx, true)),
        Expr::False => ZAst::B(Bool::from_bool(ctx, false)),
        Expr::Symbol(s) => match sorts.get(s) {
            Some(Sort::Int) | None => ZAst::I(Int::new_const(ctx, s.to_string())),
            Some(Sort::Bool) => ZAst::B(Bool::new_const(ctx, s.to_string())),
        },
        Expr::Succ([a]) => {
            let inner = build(ctx, expr, *a, sorts, cache)?.as_int().unwrap();
            ZAst::I(inner.add(&Int::from_i64(ctx, 1)))
        }
        Expr::Add([a, b]) => {
            let x = build(ctx, expr, *a, sorts, cache)?.as_int().unwrap();
            let y = build(ctx, expr, *b, sorts, cache)?.as_int().unwrap();
            ZAst::I(x.add(&y))
        }
        Expr::Mul([a, b]) => {
            let x = build(ctx, expr, *a, sorts, cache)?.as_int().unwrap();
            let y = build(ctx, expr, *b, sorts, cache)?.as_int().unwrap();
            ZAst::I(x.mul(&y))
        }
        Expr::Le([a, b]) => {
            let x = build(ctx, expr, *a, sorts, cache)?.as_int().unwrap();
            let y = build(ctx, expr, *b, sorts, cache)?.as_int().unwrap();
            ZAst::B(x.le(&y))
        }
        Expr::Not([a]) => {
            let x = build(ctx, expr, *a, sorts, cache)?.as_bool().unwrap();
            ZAst::B(x.not())
        }
        Expr::And([a, b]) => {
            let x = build(ctx, expr, *a, sorts, cache)?.as_bool().unwrap();
            let y = build(ctx, expr, *b, sorts, cache)?.as_bool().unwrap();
            ZAst::B(x.and(&[&y]))
        }
        Expr::Or([a, b]) => {
            let x = build(ctx, expr, *a, sorts, cache)?.as_bool().unwrap();
            let y = build(ctx, expr, *b, sorts, cache)?.as_bool().unwrap();
            ZAst::B(x.or(&[&y]))
        }
        Expr::Implies([a, b]) => {
            let x = build(ctx, expr, *a, sorts, cache)?.as_bool().unwrap();
            let y = build(ctx, expr, *b, sorts, cache)?.as_bool().unwrap();
            ZAst::B(x.implies(&y))
        }
    };
    cache.insert(id, match &value {
        ZAst::I(i) => ZAst::I(i.clone()),
        ZAst::B(b) => ZAst::B(b.clone()),
    });
    Ok(value)
}

/// The root of a `RecExpr` is its last node (children are added first).
fn root_id(expr: &RecExpr<Expr>) -> Id {
    Id::from(expr.as_ref().len() - 1)
}

/// Prove `goal` (both sides equal) with Z3: assert the negation and check
/// for unsatisfiability.
pub fn smt_prove(goal: &Goal) -> Result<(), SmtError> {
    let lhs = RecExpr::<Expr>::from_str(&goal.lhs).map_err(|e| ParseError(e.to_string()))?;
    let rhs = RecExpr::<Expr>::from_str(&goal.rhs).map_err(|e| ParseError(e.to_string()))?;
    smt_prove_parsed(&lhs, &rhs)
}

/// Prove that two already-parsed `Expr` trees are equal, with Z3.
pub fn smt_prove_parsed(lhs: &RecExpr<Expr>, rhs: &RecExpr<Expr>) -> Result<(), SmtError> {
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);

    let mut sorts = HashMap::new();
    let lhs_sort = type_tree(lhs, root_id(lhs), &mut sorts)?;
    let rhs_sort = type_tree(rhs, root_id(rhs), &mut sorts)?;
    if lhs_sort != rhs_sort {
        return Err(SmtError::KindMismatch {
            lhs: lhs_sort,
            rhs: rhs_sort,
        });
    }

    let mut cache = HashMap::new();
    let lhs_ast = build(&ctx, lhs, root_id(lhs), &sorts, &mut cache)?;
    let rhs_ast = build(&ctx, rhs, root_id(rhs), &sorts, &mut cache)?;

    let disequality: Bool = match (lhs_ast, rhs_ast) {
        (ZAst::I(a), ZAst::I(b)) => a._eq(&b).not(),
        (ZAst::B(a), ZAst::B(b)) => a._eq(&b).not(),
        _ => {
            return Err(SmtError::KindMismatch {
                lhs: lhs_sort,
                rhs: rhs_sort,
            })
        }
    };
    solver.assert(&disequality);
    match solver.check() {
        SatResult::Unsat => Ok(()),
        SatResult::Sat => Err(SmtError::Counterexample),
        SatResult::Unknown => Err(SmtError::Unknown),
    }
}

/// Smoke-test that a Z3 context and solver can be created and queried.
/// Returns `true` iff Z3 reports the empty conjunction (trivially) satisfiable.
pub fn smt_available() -> bool {
    let cfg = Config::new();
    let ctx = Context::new(&cfg);
    let solver = Solver::new(&ctx);
    matches!(solver.check(), SatResult::Sat)
}
