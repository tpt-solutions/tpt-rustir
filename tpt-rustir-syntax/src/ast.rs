//! Surface concrete syntax tree, with named binders and source spans.

use std::ops::Range;

pub type Span = Range<usize>;

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Var(String),
    NatLit(u64),
    /// `(x : A) -> B`, or a non-dependent arrow desugars to `Pi("_", A, B)`.
    Pi(String, Box<Expr>, Box<Expr>),
    /// `(x : A) * B`, or a non-dependent product desugars to `Sigma("_", A, B)`.
    Sigma(String, Box<Expr>, Box<Expr>),
    Lambda(String, Box<Expr>, Box<Expr>),
    App(Box<Expr>, Box<Expr>),
    Pair(Box<Expr>, Box<Expr>),
    Let(String, Box<Expr>, Box<Expr>, Box<Expr>),
    Ann(Box<Expr>, Box<Expr>),
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Expr { kind, span }
    }
}
