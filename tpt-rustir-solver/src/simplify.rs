//! Equality saturation entry points: simplify a single expression, or check
//! whether two expressions are provably equal under a rule set.

use std::str::FromStr;

use egg::{AstSize, Extractor, RecExpr, Runner};

use crate::lang::{Expr, Rw};

#[derive(Debug, Clone)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to parse expression: {}", self.0)
    }
}
impl std::error::Error for ParseError {}

pub fn parse(src: &str) -> Result<RecExpr<Expr>, ParseError> {
    RecExpr::from_str(src).map_err(|e| ParseError(e.to_string()))
}

/// Simplify `expr` to its smallest equivalent form (by AST size) after
/// running equality saturation with `rules`.
pub fn simplify(expr: &RecExpr<Expr>, rules: &[Rw]) -> RecExpr<Expr> {
    let runner = Runner::default().with_expr(expr).run(rules);
    let extractor = Extractor::new(&runner.egraph, AstSize);
    let (_, best) = extractor.find_best(runner.roots[0]);
    best
}

/// Check whether `lhs` and `rhs` end up in the same equivalence class after
/// saturating with `rules` — i.e. whether the rules prove them equal.
pub fn prove_equal(lhs: &RecExpr<Expr>, rhs: &RecExpr<Expr>, rules: &[Rw]) -> bool {
    let runner = Runner::default().with_expr(lhs).with_expr(rhs).run(rules);
    let lhs_id = runner.egraph.find(runner.roots[0]);
    let rhs_id = runner.egraph.find(runner.roots[1]);
    lhs_id == rhs_id
}
